import { useCallback, useState } from 'react';

const MODE_TO_FIELDS = {
  range: ['start', 'end'],
  key: ['key'],
  prefix: ['keyPrefix']
};

const DEFAULT_MODES = ['range', 'key', 'prefix'];

function determineInitialMode(value = {}) {
  if (value.start || value.end) return 'range';
  if (value.key) return 'key';
  if (value.keyPrefix) return 'prefix';
  return 'range';
}

function clearFieldsOutsideMode(value, mode) {
  const allowed = new Set(MODE_TO_FIELDS[mode] ?? []);
  return Object.fromEntries(Object.entries(value).filter(([field]) => allowed.has(field)));
}

export function useRangeMode(initialValue = {}, onChange, allowedModes = DEFAULT_MODES) {
  const [activeMode, setActiveMode] = useState(() => determineInitialMode(initialValue));
  const [value, setValue] = useState(initialValue);

  const emit = useCallback(
    (nextValue) => {
      setValue(nextValue);
      onChange?.(nextValue);
    },
    [onChange]
  );

  const changeMode = useCallback(
    (newMode) => {
      if (!allowedModes.includes(newMode)) return;
      setActiveMode(newMode);
      emit({});
    },
    [allowedModes, emit]
  );

  const updateValue = useCallback(
    (field, newValue) => {
      const baseMode = field === 'start' || field === 'end' ? 'range' : field === 'key' ? 'key' : 'prefix';
      const nextMode = MODE_TO_FIELDS[activeMode]?.includes(field) ? activeMode : baseMode;
      setActiveMode(nextMode);
      emit({ ...clearFieldsOutsideMode(value, nextMode), [field]: newValue });
    },
    [activeMode, emit, value]
  );

  const clearValue = useCallback(() => emit({}), [emit]);

  const setEntireValue = useCallback(
    (newValue) => {
      setActiveMode(determineInitialMode(newValue));
      emit(newValue);
    },
    [emit]
  );

  return {
    state: { activeMode, value },
    actions: { changeMode, updateValue, clearValue, setValue: setEntireValue },
    getAvailableModes: useCallback(() => allowedModes, [allowedModes]),
    isValidMode: useCallback((mode) => allowedModes.includes(mode), [allowedModes])
  };
}

export default useRangeMode;
