import { formatUnknownError } from './error-utils'

const INVALID_ARGUMENT_MARKERS = [
  'invalid args',
  'missing required key',
  'missing field',
  'invalid type',
  'failed to deserialize',
]

function normalizeCommand(command: string) {
  return command.trim().toLowerCase()
}

function buildUnavailableMarkers(command: string) {
  return [
    `unknown command ${command}`,
    `command ${command} not found`,
    `command "${command}" not found`,
    `command '${command}' not found`,
    `command \`${command}\` not found`,
  ]
}

export function isTauriCommandUnavailableError(error: unknown, command: string): boolean {
  const normalizedCommand = normalizeCommand(command)
  const message = formatUnknownError(error, '').toLowerCase()

  if (!message || !message.includes(normalizedCommand)) {
    return false
  }

  if (INVALID_ARGUMENT_MARKERS.some((marker) => message.includes(marker))) {
    return false
  }

  return buildUnavailableMarkers(normalizedCommand).some((marker) => message.includes(marker))
}
