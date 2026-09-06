import { useState } from "react";
import { SettingRow } from "../components/settings/SettingRow";
import { SettingSection } from "../components/settings/SettingSection";
import { SettingsDialog } from "../components/settings/SettingsDialog";
import { Switch } from "../components/settings/Switch";
import "./SettingsVisualHarness.css";

type SettingsVisualHarnessProps = { initialDialogOpen?: boolean };

export function SettingsVisualHarness({ initialDialogOpen = false }: SettingsVisualHarnessProps) {
  const [switchOff, setSwitchOff] = useState(false);
  const [switchOn, setSwitchOn] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(initialDialogOpen);
  const [noticeVisible, setNoticeVisible] = useState(true);

  return <main className="visual-harness" aria-labelledby="visual-harness-title">
    <header className="visual-harness-header">
      <div><p>Development Visual Harness</p><h1 id="visual-harness-title">设置型组件视觉样本</h1></div>
      <strong>仅开发环境，不代表产品设置</strong>
    </header>
    <div className="visual-harness-canvas">
      <div className="settings-grid">
        <div className="settings-column">
          <SettingSection title="SettingRow 样本">
            <SettingRow label="基础行" value="默认状态" />
            <SettingRow label="带补充说明的行" description="此文字仅用于验证 secondary text 的间距与换行。" value="示例值" />
            <SettingRow label="Switch OFF" control={<Switch label="Switch OFF visual fixture" checked={switchOff} onCheckedChange={setSwitchOff} />} />
            <SettingRow label="Switch ON" control={<Switch label="Switch ON visual fixture" checked={switchOn} onCheckedChange={setSwitchOn} />} />
            <SettingRow label="Switch Disabled" control={<Switch label="Switch Disabled visual fixture" checked disabled onCheckedChange={() => undefined} />} />
          </SettingSection>
          <SettingSection title="字段样本">
            <SettingRow label="Select" control={<select aria-label="Select visual fixture" defaultValue="default"><option value="default">默认选项</option><option value="alternative">备选项</option></select>} />
            <SettingRow label="Input" control={<input aria-label="Input visual fixture" defaultValue="示例输入" />} />
            <SettingRow label="Inline Loading" control={<span className="setting-row-loading" role="status">正在处理…</span>} />
          </SettingSection>
        </div>
        <div className="settings-column">
          <SettingSection title="动作样本">
            <SettingRow label="Chevron Row" value={<span className="visual-harness-chevron" aria-hidden="true">&#xeab6;</span>} onActivate={() => setDialogOpen(true)} />
            <SettingRow label="Button" control={<div className="settings-action-group"><button type="button" className="primary-button">主要按钮</button><button type="button" className="secondary-button">次要按钮</button></div>} />
            <SettingRow label="IconButton" control={<button type="button" className="visual-harness-icon-button" aria-label="IconButton visual fixture"><span className="visual-harness-codicon" aria-hidden="true">&#xeb51;</span></button>} />
            <SettingRow label="Disabled Button" control={<button type="button" className="primary-button" disabled>禁用按钮</button>} />
          </SettingSection>
          <SettingSection title="Notice 样本">
            <SettingRow label="Notice" description="使用本地 fixture state 验证 popup surface、边框和关闭动作。" control={<button type="button" className="secondary-button" onClick={() => setNoticeVisible(true)}>显示</button>} />
          </SettingSection>
        </div>
      </div>
    </div>
    {noticeVisible ? <div className="subscription-notice subscription-notice-success" role="status"><span>Visual fixture notice</span><button type="button" className="visual-harness-notice-close" onClick={() => setNoticeVisible(false)}>关闭</button></div> : null}
    {dialogOpen ? <SettingsDialog title="Dialog visual fixture" onClose={() => setDialogOpen(false)}>
      <p className="visual-harness-dialog-copy">此弹窗仅验证视觉壳、焦点顺序与关闭行为，不映射任何产品设置。</p>
      <label className="visual-harness-dialog-field">Input<input defaultValue="Dialog fixture" /></label>
    </SettingsDialog> : null}
  </main>;
}
