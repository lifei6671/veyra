type SwitchProps = {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onCheckedChange: (checked: boolean) => void;
};

export function Switch({ checked, disabled = false, label, onCheckedChange }: SwitchProps) {
  return (
    <label className="veyra-switch">
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={checked}
        disabled={disabled}
        onChange={(event) => onCheckedChange(event.currentTarget.checked)}
      />
      <span className="veyra-switch-track" aria-hidden="true">
        <span className="veyra-switch-thumb" />
      </span>
    </label>
  );
}
