import type { AudioDevice } from "../lib/recorder";

type Props = {
  label: string;
  devices: AudioDevice[];
  enabled: boolean;
  deviceId: string | null;
  onEnabledChange: (enabled: boolean) => void;
  onDeviceChange: (id: string) => void;
};

export function AudioSource({ label, devices, enabled, deviceId, onEnabledChange, onDeviceChange }: Props) {
  return (
    <div className="source">
      <label className="toggle">
        <input
          type="checkbox"
          checked={enabled}
          disabled={devices.length === 0}
          onChange={(event) => onEnabledChange(event.target.checked)}
        />
        {label}
      </label>
      <select
        aria-label={`Dispositivo de ${label.toLowerCase()}`}
        value={deviceId ?? ""}
        disabled={!enabled}
        onChange={(event) => onDeviceChange(event.target.value)}
      >
        {devices.map((device) => (
          <option key={device.id} value={device.id}>
            {device.name}
          </option>
        ))}
      </select>
    </div>
  );
}
