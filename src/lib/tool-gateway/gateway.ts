import {
  runtimeToolCreate,
  runtimeToolDelete,
  runtimeToolEdit,
  runtimeToolGrep,
  runtimeToolLs,
  runtimeToolMove,
  runtimeToolRead,
} from '@/platform/tauri/clients/tool-runtime-client'

import type {
  ToolCreateInput,
  ToolDeleteInput,
  ToolEditInput,
  ToolGateway,
  ToolGrepInput,
  ToolMoveInput,
  ToolReadInput,
} from './types'
import type { ToolLsInput } from './ls-types'

class RuntimeToolGateway implements ToolGateway {
  create(input: ToolCreateInput) {
    return runtimeToolCreate(input)
  }

  read(input: ToolReadInput) {
    return runtimeToolRead(input)
  }

  edit(input: ToolEditInput) {
    return runtimeToolEdit(input)
  }

  delete(input: ToolDeleteInput) {
    return runtimeToolDelete(input)
  }

  move(input: ToolMoveInput) {
    return runtimeToolMove(input)
  }

  ls(input: ToolLsInput) {
    return runtimeToolLs(input)
  }

  grep(input: ToolGrepInput) {
    return runtimeToolGrep(input)
  }
}

export function createToolGateway(): ToolGateway {
  return new RuntimeToolGateway()
}

export const toolGateway = createToolGateway()
