// Adapted from shadcn/ui (MIT): https://ui.shadcn.com/docs/components/radix/select
// Keep its Radix composition with the existing OpenBox theme and icons.
import type { ComponentProps, ReactNode } from "react";
import { CheckIcon, ChevronDownIcon, ChevronUpIcon } from "@heroicons/react/24/outline";
import * as SelectPrimitive from "@radix-ui/react-select";

export const Select = SelectPrimitive.Root;
export const SelectGroup = SelectPrimitive.Group;
export const SelectValue = SelectPrimitive.Value;

export function SelectTrigger({ className = "", children, ...props }: ComponentProps<typeof SelectPrimitive.Trigger>) {
  return <SelectPrimitive.Trigger data-slot="select-trigger" className={`ob-select-trigger ${className}`.trim()} {...props}>
    {children}<SelectPrimitive.Icon asChild><ChevronDownIcon aria-hidden="true" /></SelectPrimitive.Icon>
  </SelectPrimitive.Trigger>;
}

export function SelectContent({ children, className = "", header, portalContainer, ...props }: ComponentProps<typeof SelectPrimitive.Content> & { header?: ReactNode; portalContainer?: HTMLElement }) {
  return <SelectPrimitive.Portal container={portalContainer}><SelectPrimitive.Content data-slot="select-content" className={`ob-select-content ${className}`.trim()} position="popper" sideOffset={4} {...props}>
    {header}
    <SelectPrimitive.ScrollUpButton className="ob-select-scroll"><ChevronUpIcon aria-hidden="true" /></SelectPrimitive.ScrollUpButton>
    <SelectPrimitive.Viewport className="ob-select-viewport">{children}</SelectPrimitive.Viewport>
    <SelectPrimitive.ScrollDownButton className="ob-select-scroll"><ChevronDownIcon aria-hidden="true" /></SelectPrimitive.ScrollDownButton>
  </SelectPrimitive.Content></SelectPrimitive.Portal>;
}

export function SelectLabel(props: ComponentProps<typeof SelectPrimitive.Label>) {
  return <SelectPrimitive.Label data-slot="select-label" className="ob-select-label" {...props} />;
}

export function SelectItem({ children, ...props }: ComponentProps<typeof SelectPrimitive.Item>) {
  return <SelectPrimitive.Item data-slot="select-item" className="ob-select-item" {...props}>
    <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
    <SelectPrimitive.ItemIndicator className="ob-select-indicator"><CheckIcon aria-hidden="true" /></SelectPrimitive.ItemIndicator>
  </SelectPrimitive.Item>;
}
