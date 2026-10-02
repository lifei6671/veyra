import { useState, type ComponentProps, type ReactNode } from "react";
import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "./select";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import * as ToggleGroupPrimitive from "@radix-ui/react-toggle-group";

const join = (...classes: Array<string | undefined | false>) => classes.filter(Boolean).join(" ");

export type SelectOption = {
  value: string;
  label: string;
  group?: string;
  disabled?: boolean;
};

export function SelectControl({
  label,
  options,
  value,
  defaultValue,
  onValueChange,
  className,
  contentClassName,
  disabled,
  placeholder,
}: {
  label: string;
  options: SelectOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  className?: string;
  contentClassName?: string;
  disabled?: boolean;
  placeholder?: string;
}) {
  const [selected, setSelected] = useState(defaultValue);
  const current = value ?? selected;
  const groups = [...new Set(options.map(option => option.group))];
  const encode = (item: string | undefined) => item === "" ? "__openbox_empty__" : item;
  return <Select value={encode(current)} disabled={disabled} onValueChange={next => {
    const decoded = next === "__openbox_empty__" ? "" : next;
    if (value === undefined) setSelected(decoded);
    onValueChange?.(decoded);
  }}>
    <SelectTrigger className={className} aria-label={label}>
      <SelectValue placeholder={placeholder}>{options.find(option => option.value === current)?.label ?? placeholder ?? current}</SelectValue>
    </SelectTrigger>
    <SelectContent className={contentClassName}>{groups.map(group => <SelectGroup key={group ?? "ungrouped"}>
      {group && <SelectLabel>{group}</SelectLabel>}
      {options.filter(option => option.group === group).map(option => <SelectItem value={encode(option.value)!} key={option.value} disabled={option.disabled}>{option.label}</SelectItem>)}
    </SelectGroup>)}</SelectContent>
  </Select>;
}

export function SwitchControl({ label, checked, defaultChecked, onCheckedChange, className, disabled }: {
  label: string;
  checked?: boolean;
  defaultChecked?: boolean;
  disabled?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  className?: string;
}) {
  return <SwitchPrimitive.Root
    className={join("ob-switch", className)}
    aria-label={label}
    disabled={disabled}
    checked={checked}
    defaultChecked={defaultChecked}
    onCheckedChange={onCheckedChange}
  >
    <SwitchPrimitive.Thumb className="ob-switch-thumb" />
  </SwitchPrimitive.Root>;
}

export function SegmentedGroup({ className, value, onValueChange, ...props }: Omit<ComponentProps<typeof ToggleGroupPrimitive.Root>, "type" | "value" | "defaultValue" | "onValueChange"> & { value: string; onValueChange: (value: string) => void }) {
  return <ToggleGroupPrimitive.Root
    type="single"
    className={join("ob-segmented-group", className)}
    value={value}
    onValueChange={next => { if (next) onValueChange(next); }}
    {...props}
  />;
}

export const SegmentedItem = ({ className, ...props }: ComponentProps<typeof ToggleGroupPrimitive.Item>) => <ToggleGroupPrimitive.Item className={join("ob-segmented-item", className)} {...props} />;

export function SearchInputGroup({
  label,
  value,
  onChange,
  onClear,
  placeholder,
  className,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
  placeholder: string;
  className?: string;
}) {
  return <div className={join("ob-input-group", "proxy-search", className)}>
    <span className="ob-input-addon"><MagnifyingGlassIcon aria-hidden="true" /></span>
    <input aria-label={label} value={value} onChange={event => onChange(event.target.value)} placeholder={placeholder} />
    <button className="ob-input-action" type="button" aria-label={`清空${label}`} onClick={onClear}>×</button>
  </div>;
}

export function SwitchLabel({ label, children }: { label: string; children: ReactNode }) {
  return <span className="switch-label"><span>{label}</span>{children}</span>;
}
