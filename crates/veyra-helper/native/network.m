#import <Foundation/Foundation.h>
#import <SystemConfiguration/SystemConfiguration.h>
#include <libproc.h>
#include <sys/proc_info.h>
#include <fcntl.h>
#include <unistd.h>

// This adapter is private to the prototype. No service ID or dictionary comes
// from IPC. Rust uses only the ID of the dedicated service it created.
static char *encode(NSDictionary *value) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:NSJSONWritingSortedKeys error:nil];
    if (!data) return strdup("{\"error\":\"JSON encoding failed\"}");
    NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    return strdup(text.UTF8String);
}
static NSDictionary *failure(NSString *operation) {
    int code = SCError();
    return @{ @"error": operation, @"sc_error": @(code),
              @"detail": [NSString stringWithUTF8String:SCErrorString(code)] };
}
static BOOL isolated(SCPreferencesRef prefs, SCNetworkServiceRef service) {
    if (!service || SCNetworkServiceGetEnabled(service)) return NO;
    CFStringRef name = SCNetworkServiceGetName(service);
    if (!name || !CFEqual(name, CFSTR("Veyra-P005-Isolated-Disabled"))) return NO;
    CFArrayRef sets = SCNetworkSetCopyAll(prefs);
    BOOL safe = YES;
    if (sets) {
        for (id set in (__bridge NSArray *)sets) {
            CFArrayRef services = SCNetworkSetCopyServices((__bridge SCNetworkSetRef)set);
            if (services) {
                for (id item in (__bridge NSArray *)services)
                    if (CFEqual(SCNetworkServiceGetServiceID((__bridge SCNetworkServiceRef)item),
                                SCNetworkServiceGetServiceID(service))) safe = NO;
                CFRelease(services);
            }
        }
        CFRelease(sets);
    }
    return safe;
}
static BOOL commit(SCPreferencesRef prefs) {
    return SCPreferencesCommitChanges(prefs) && SCPreferencesApplyChanges(prefs);
}

char *p005_network(const char *operation, const char *identifier, const char *json) {
    @autoreleasepool {
        NSString *op = [NSString stringWithUTF8String:operation];
        SCPreferencesRef prefs = SCPreferencesCreate(NULL, CFSTR("Veyra-P005"), NULL);
        if (!prefs) return encode(failure(@"SCPreferencesCreate"));
        BOOL locked = [op isEqual:@"read"] ? NO : SCPreferencesLock(prefs, FALSE);
        if (![op isEqual:@"read"] && !locked) {
            NSDictionary *result = failure(@"SCPreferencesLock"); CFRelease(prefs); return encode(result);
        }
        SCNetworkServiceRef service = NULL;
        NSDictionary *result = nil;
        if ([op isEqual:@"create"]) {
            CFArrayRef interfaces = SCNetworkInterfaceCopyAll();
            if (interfaces) {
                for (id item in (__bridge NSArray *)interfaces) {
                    SCNetworkInterfaceRef interface = (__bridge SCNetworkInterfaceRef)item;
                    CFStringRef type = SCNetworkInterfaceGetInterfaceType(interface);
                    if (CFEqual(type, kSCNetworkInterfaceTypeEthernet) || CFEqual(type, kSCNetworkInterfaceTypeIEEE80211)) {
                        service = SCNetworkServiceCreate(prefs, interface); break;
                    }
                }
                CFRelease(interfaces);
            }
            if (!service) result = failure(@"SCNetworkServiceCreate physical interface");
            else if (!SCNetworkServiceSetName(service, CFSTR("Veyra-P005-Isolated-Disabled")) ||
                     !SCNetworkServiceSetEnabled(service, FALSE)) result = failure(@"name/disable test service");
            else {
                // Do not establish IPv4/IPv6, add to any set, or enable it.
                if (!SCNetworkServiceAddProtocolType(service, kSCNetworkProtocolTypeProxies))
                    result = failure(@"SCNetworkServiceAddProtocolType Proxies");
                else if (!isolated(prefs, service)) result = @{ @"error": @"service is not isolated" };
                else {
                    NSString *identifier = (__bridge NSString *)SCNetworkServiceGetServiceID(service);
                    int fd = open("/Library/Application Support/VeyraP005/service-id", O_WRONLY|O_CREAT|O_EXCL, 0600);
                    if (fd < 0) result = @{ @"error": @"persist service ID before commit", @"errno": @(errno) };
                    else {
                        const char *text = identifier.UTF8String;
                        BOOL saved = write(fd, text, strlen(text)) == (ssize_t)strlen(text) && fsync(fd) == 0;
                        close(fd);
                        if (!saved) result = @{ @"error": @"service ID journal write/fsync failed" };
                        else if (!commit(prefs)) result = failure(@"commit/apply create");
                        else result = @{ @"id": identifier, @"enabled": @NO, @"in_any_network_set": @NO };
                    }
                }
            }
        } else {
            service = SCNetworkServiceCopy(prefs, (__bridge CFStringRef)[NSString stringWithUTF8String:identifier]);
            if (!service && [op isEqual:@"delete"]) result = @{ @"deleted": @YES, @"already_absent": @YES };
            else if (!isolated(prefs, service)) result = @{ @"error": @"recorded service missing or isolation changed; no write" };
            else if ([op isEqual:@"delete"]) {
                if (!SCNetworkServiceRemove(service) || !commit(prefs)) result = failure(@"remove/commit/apply");
                else result = @{ @"deleted": @YES };
            } else {
                SCNetworkProtocolRef protocol = SCNetworkServiceCopyProtocol(service, kSCNetworkProtocolTypeProxies);
                if (!protocol) result = failure(@"SCNetworkServiceCopyProtocol Proxies");
                else {
                    if ([op isEqual:@"write"]) {
                        NSData *data = [[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding];
                        id request = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
                        id config = [request isKindOfClass:NSDictionary.class] ? request[@"desired"] : nil;
                        id expected = [request isKindOfClass:NSDictionary.class] ? request[@"expected"] : nil;
                        CFDictionaryRef dict = SCNetworkProtocolGetConfiguration(protocol);
                        NSDictionary *current = dict ? (__bridge NSDictionary *)dict : @{};
                        if (![config isKindOfClass:NSDictionary.class] || ![expected isKindOfClass:NSDictionary.class])
                            result = @{ @"error": @"internal write/expected dictionary invalid" };
                        else if (![current isEqual:expected]) result = @{ @"error": @"proxy changed since observation; no write" };
                        else if (!SCNetworkProtocolSetConfiguration(protocol, (__bridge CFDictionaryRef)config) || !commit(prefs))
                            result = failure(@"set/commit/apply proxies");
                    }
                    if (!result) {
                        CFDictionaryRef dict = SCNetworkProtocolGetConfiguration(protocol);
                        result = @{ @"id": (__bridge NSString *)SCNetworkServiceGetServiceID(service),
                                    @"enabled": @NO, @"in_any_network_set": @NO,
                                    @"proxies": dict ? (__bridge NSDictionary *)dict : @{} };
                    }
                    CFRelease(protocol);
                }
            }
        }
        if (service) CFRelease(service);
        if (locked) SCPreferencesUnlock(prefs);
        CFRelease(prefs);
        return encode(result ?: @{ @"error": @"unknown internal network operation" });
    }
}

char *p005_process(int pid) {
    struct proc_bsdinfo info = {0};
    int size = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof(info));
    if (size != sizeof(info)) return encode(@{ @"error": @"proc_pidinfo PROC_PIDTBSDINFO", @"errno": @(errno) });
    return encode(@{ @"pid": @(info.pbi_pid), @"uid": @(info.pbi_uid), @"ruid": @(info.pbi_ruid),
                     @"gid": @(info.pbi_gid), @"rgid": @(info.pbi_rgid), @"pgid": @(info.pbi_pgid),
                     @"start_sec": @(info.pbi_start_tvsec), @"start_usec": @(info.pbi_start_tvusec) });
}
