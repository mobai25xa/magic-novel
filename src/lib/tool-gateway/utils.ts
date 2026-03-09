import { nanoid } from 'nanoid'

export function createCallId(prefix = 'call'): string {
  return `${prefix}_${Date.now()}_${nanoid(10)}`
}
