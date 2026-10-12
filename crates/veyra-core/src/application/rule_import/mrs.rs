//! MRSv1：按 Mihomo mrs_reader/domain_set_bin/ipcidr_set_bin 的正式格式读取。
//! 只提取可等价导入的域名/IP 明细，不实现新的代理内核或规则匹配器。
use super::{MAX_DECODED_BYTES, RuleImportEntry, RuleImportError, RuleImportKind, entry};
use std::{
    collections::BTreeSet,
    io::{Cursor, Read},
    net::{Ipv4Addr, Ipv6Addr},
};
// 格式来源：https://github.com/MetaCubeX/mihomo/blob/Meta/rules/provider/mrs_reader.go
// https://github.com/MetaCubeX/mihomo/blob/Meta/component/trie/domain_set_bin.go
// https://github.com/MetaCubeX/mihomo/blob/Meta/component/cidr/ipcidr_set_bin.go
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<RuleImportEntry>, RuleImportError> {
    let mut reader = ruzstd::decoding::StreamingDecoder::new(bytes)
        .map_err(|_| RuleImportError::InvalidFormat)?;
    let mut decoded = Vec::new();
    reader
        .by_ref()
        .take(MAX_DECODED_BYTES as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| RuleImportError::InvalidFormat)?;
    if decoded.len() > MAX_DECODED_BYTES {
        return Err(RuleImportError::BodyTooLarge);
    }
    // 一个完整 frame 的校验和与结束位置都须一致，不接受坏 checksum 或附加 binary。
    if !reader.get_ref().is_empty()
        || reader
            .decoder
            .get_checksum_from_data()
            .is_some_and(|expected| reader.decoder.get_calculated_checksum() != Some(expected))
    {
        return Err(RuleImportError::InvalidFormat);
    }
    let mut input = Cursor::new(decoded.as_slice());
    if read::<4>(&mut input)? != *b"MRS\x01" {
        return Err(RuleImportError::InvalidFormat);
    }
    let behavior = read::<1>(&mut input)?[0];
    let count = length(&mut input)?;
    if count == 0 {
        return Err(RuleImportError::InvalidFormat);
    }
    let extra = length(&mut input)?;
    skip(&mut input, extra)?;
    let entries = match behavior {
        0 => domains(&mut input)?,
        1 => ranges(&mut input)?,
        _ => return Err(RuleImportError::UnsupportedFormat),
    };
    if input.position() as usize != decoded.len() {
        return Err(RuleImportError::InvalidFormat);
    }
    Ok(entries)
}
fn read<const N: usize>(input: &mut Cursor<&[u8]>) -> Result<[u8; N], RuleImportError> {
    let mut bytes = [0u8; N];
    input
        .read_exact(&mut bytes)
        .map_err(|_| RuleImportError::InvalidFormat)?;
    Ok(bytes)
}
fn length(input: &mut Cursor<&[u8]>) -> Result<usize, RuleImportError> {
    let n = i64::from_be_bytes(read::<8>(input)?);
    usize::try_from(n).map_err(|_| RuleImportError::InvalidFormat)
}
fn skip(input: &mut Cursor<&[u8]>, len: usize) -> Result<(), RuleImportError> {
    let end = (input.position() as usize)
        .checked_add(len)
        .ok_or(RuleImportError::InvalidFormat)?;
    if end > input.get_ref().len() {
        return Err(RuleImportError::InvalidFormat);
    }
    input.set_position(end as u64);
    Ok(())
}
fn bitmap(input: &mut Cursor<&[u8]>) -> Result<Vec<u64>, RuleImportError> {
    let n = length(input)?;
    if n == 0
        || n > input
            .get_ref()
            .len()
            .saturating_sub(input.position() as usize)
            / 8
    {
        return Err(RuleImportError::InvalidFormat);
    }
    (0..n)
        .map(|_| Ok(u64::from_be_bytes(read::<8>(input)?)))
        .collect()
}
fn bit(map: &[u64], index: usize) -> Result<bool, RuleImportError> {
    let word = map.get(index / 64).ok_or(RuleImportError::InvalidFormat)?;
    Ok(word & (1 << (index % 64)) != 0)
}
fn domains(input: &mut Cursor<&[u8]>) -> Result<Vec<RuleImportEntry>, RuleImportError> {
    if read::<1>(input)?[0] != 1 {
        return Err(RuleImportError::InvalidFormat);
    }
    let leaves = bitmap(input)?;
    let labels_bitmap = bitmap(input)?;
    let n = length(input)?;
    let start = input.position() as usize;
    skip(input, n)?;
    let labels = &input.get_ref()[start..start + n];
    // LOUDS 的 node 顺序是 BFS；记录真实 parent/label，以迭代遍历避免外部数据递归栈。
    let mut nodes = vec![(0usize, 0u8, 0usize)];
    let mut bitmap_index = 0;
    let mut label_index = 0;
    let mut node_index = 0;
    while node_index < nodes.len() {
        while !bit(&labels_bitmap, bitmap_index)? {
            let label = *labels
                .get(label_index)
                .ok_or(RuleImportError::InvalidFormat)?;
            let depth = nodes[node_index].2 + 1;
            if depth > 1024 {
                return Err(RuleImportError::InvalidFormat);
            }
            nodes.push((node_index, label, depth));
            label_index += 1;
            bitmap_index += 1;
        }
        bitmap_index += 1;
        node_index += 1;
    }
    if label_index != labels.len() || bit(&leaves, 0)? {
        return Err(RuleImportError::InvalidFormat);
    }
    let mut patterns = BTreeSet::new();
    for index in 1..nodes.len() {
        if bit(&leaves, index)? {
            let mut reversed = Vec::new();
            let mut current = index;
            while current != 0 {
                reversed.push(nodes[current].1);
                current = nodes[current].0;
            }
            reversed.reverse();
            let key = std::str::from_utf8(&reversed).map_err(|_| RuleImportError::InvalidFormat)?;
            let pattern = key.chars().rev().collect::<String>();
            patterns.insert(pattern);
        }
    }
    let mut result = Vec::new();
    for pattern in &patterns {
        if let Some(domain) = pattern
            .strip_prefix("+.")
            .or_else(|| pattern.strip_prefix('.'))
        {
            // suffix-only 模式不等于 React domainSuffix（包含根域名）；只有实际 exact 配对才合并。
            if !patterns.contains(domain) || domain.contains(['*', '+']) {
                return Err(RuleImportError::UnsupportedRule);
            }
            result.push(entry(RuleImportKind::DomainSuffix, domain)?);
        } else if pattern.contains(['*', '+']) {
            return Err(RuleImportError::UnsupportedRule);
        } else if !patterns.contains(&format!("+.{pattern}"))
            && !patterns.contains(&format!(".{pattern}"))
        {
            result.push(entry(RuleImportKind::Domain, pattern)?);
        }
    }
    result.sort_by(|a, b| a.value.cmp(&b.value));
    result.dedup();
    Ok(result)
}
fn ranges(input: &mut Cursor<&[u8]>) -> Result<Vec<RuleImportEntry>, RuleImportError> {
    if read::<1>(input)?[0] != 1 {
        return Err(RuleImportError::InvalidFormat);
    }
    let count = length(input)?;
    if count == 0
        || count
            > input
                .get_ref()
                .len()
                .saturating_sub(input.position() as usize)
                / 32
    {
        return Err(RuleImportError::InvalidFormat);
    }
    let mut result = Vec::new();
    for _ in 0..count {
        let start = Ipv6Addr::from(read::<16>(input)?);
        let end = Ipv6Addr::from(read::<16>(input)?);
        let (mut first, last, bits) = match (start.to_ipv4_mapped(), end.to_ipv4_mapped()) {
            (Some(a), Some(b)) => (u32::from(a) as u128, u32::from(b) as u128, 32),
            (None, None) => (u128::from(start), u128::from(end), 128),
            _ => return Err(RuleImportError::InvalidFormat),
        };
        if first > last {
            return Err(RuleImportError::InvalidFormat);
        }
        loop {
            let room = last - first;
            let fitting = if room == u128::MAX {
                128
            } else {
                127 - (room + 1).leading_zeros()
            };
            let power = first.trailing_zeros().min(bits).min(fitting);
            let address = if bits == 32 {
                Ipv4Addr::from(first as u32).to_string()
            } else {
                Ipv6Addr::from(first).to_string()
            };
            result.push(entry(
                RuleImportKind::IpCidr,
                &format!("{address}/{}", bits - power),
            )?);
            if power == 128 {
                break;
            }
            let Some(next) = first.checked_add(1u128 << power) else {
                break;
            };
            if next > last {
                break;
            }
            first = next;
        }
    }
    Ok(result)
}
