import type { ComponentProps, ReactNode } from "react";
import { CheckIcon, ChevronDownIcon, ChevronUpIcon, MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import * as SelectPrimitive from "@radix-ui/react-select";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import * as ToggleGroupPrimitive from "@radix-ui/react-toggle-group";

const join = (...classes: Array<string | undefined | false>) => classes.filter(Boolean).join(" ");

export type SelectOption = {
  value: string;
  label: string;
};

export function SelectControl({
  label,
  options,
  value,
  defaultValue,
  onValueChange,
  className,
}: {
  label: string;
  options: SelectOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  className?: string;
}) {
  return <SelectPrimitive.Root value={value} defaultValue={defaultValue} onValueChange={onValueChange}>
    <SelectPrimitive.Trigger className={join("ob-select-trigger", className)} aria-label={label}>
      <SelectPrimitive.Value />
      <SelectPrimitive.Icon asChild><ChevronDownIcon aria-hidden="true" /></SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content className="ob-select-content" position="popper" sideOffset={4}>
        <SelectPrimitive.ScrollUpButton className="ob-select-scroll"><ChevronUpIcon aria-hidden="true" /></SelectPrimitive.ScrollUpButton>
        <SelectPrimitive.Viewport className="ob-select-viewport">
          <SelectPrimitive.Group>
            {options.map(option => <SelectPrimitive.Item className="ob-select-item" value={option.value} key={option.value}>
              <SelectPrimitive.ItemText>{option.label}</SelectPrimitive.ItemText>
              <SelectPrimitive.ItemIndicator className="ob-select-indicator"><CheckIcon aria-hidden="true" /></SelectPrimitive.ItemIndicator>
            </SelectPrimitive.Item>)}
          </SelectPrimitive.Group>
        </SelectPrimitive.Viewport>
        <SelectPrimitive.ScrollDownButton className="ob-select-scroll"><ChevronDownIcon aria-hidden="true" /></SelectPrimitive.ScrollDownButton>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  </SelectPrimitive.Root>;
}

export function SwitchControl({ label, checked, defaultChecked, onCheckedChange, className }: {
  label: string;
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  className?: string;
}) {
  return <SwitchPrimitive.Root
    className={join("ob-switch", className)}
    aria-label={label}
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
