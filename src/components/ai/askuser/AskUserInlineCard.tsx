import { useMemo, useState } from 'react'

import type {
  AgentAskUserAnswer,
  AgentPendingAskUserRequest,
} from '@/agent/types'
import { AiApprovalShell, Button, Input } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'

type AskUserInlineCardProps = {
  request?: AgentPendingAskUserRequest
  onSubmit: (callId: string, answers: AgentAskUserAnswer[]) => void
  onCancel: (callId: string) => void
}

type LocalAnswerState = {
  selected: string
  custom: string
}

const CUSTOM_VALUE = '__custom__'

function createEmptyAnswerState(): LocalAnswerState {
  return {
    selected: '',
    custom: '',
  }
}

function createInitialAnswerState(questionCount: number) {
  return Array.from({ length: questionCount }, () => ({
    selected: '',
    custom: '',
  }))
}

function canSubmit(input: {
  request?: AgentPendingAskUserRequest
  states: LocalAnswerState[]
}) {
  if (!input.request) {
    return false
  }

  if (input.states.length !== input.request.questions.length) {
    return false
  }

  return input.request.questions.every((_, index) => {
    const state = input.states[index]
    if (!state || !state.selected) {
      return false
    }
    if (state.selected === CUSTOM_VALUE) {
      return Boolean(state.custom.trim())
    }
    return true
  })
}

function buildAnswers(input: {
  request: AgentPendingAskUserRequest
  states: LocalAnswerState[]
}): AgentAskUserAnswer[] {
  return input.request.questions.map((question, index) => {
    const state = input.states[index]
    const value = state.selected === CUSTOM_VALUE
      ? state.custom.trim()
      : state.selected

    return {
      topic: question.topic,
      value,
    }
  })
}

function normalizeQuestionKey(question: AgentPendingAskUserRequest['questions'][number], index: number) {
  return `${question.topic}-${index}`
}

export function AskUserInlineCard({ request, onSubmit, onCancel }: AskUserInlineCardProps) {
  const ai = useAiTranslations()

  const [answersByCallId, setAnswersByCallId] = useState<Record<string, LocalAnswerState[]>>({})

  const answers = useMemo(() => {
    if (!request) {
      return []
    }
    return answersByCallId[request.callId] || createInitialAnswerState(request.questions.length)
  }, [answersByCallId, request])

  const disabled = !canSubmit({ request, states: answers })

  const updateAnswerAt = (index: number, updater: (current: LocalAnswerState) => LocalAnswerState) => {
    if (!request) {
      return
    }

    setAnswersByCallId((prev) => {
      const existing = prev[request.callId] || createInitialAnswerState(request.questions.length)
      const nextRows = [...existing]
      const current = nextRows[index] || createEmptyAnswerState()
      nextRows[index] = updater(current)
      return {
        ...prev,
        [request.callId]: nextRows,
      }
    })
  }

  if (!request) {
    return null
  }

  return (
    <AiApprovalShell className="ai-animate-slide-in">
      <div className="space-y-1">
        <div className="text-xs font-medium">{ai.askUser.title}</div>
        <p className="text-xs text-muted-foreground">{ai.askUser.description}</p>
      </div>

      <div className="space-y-3">
        {request.questions.map((question, index) => {
          const row = answers[index] || createEmptyAnswerState()
          const groupName = `askuser-inline-${request.callId}-${index}`

          return (
            <section
              key={normalizeQuestionKey(question, index)}
              className="space-y-2 rounded-md border border-border p-2.5 bg-background-50"
            >
              <div className="text-xs text-muted-foreground">{index + 1}. {question.topic}</div>
              <div className="text-sm font-medium leading-relaxed">{question.question}</div>

              <div className="space-y-1.5">
                {question.options.map((option, optionIndex) => (
                  <label
                    key={`${option}-${optionIndex}`}
                    className="flex items-start gap-2 text-sm cursor-pointer"
                  >
                    <input
                      type="radio"
                      name={groupName}
                      className="mt-1"
                      checked={row.selected === option}
                      onChange={() => {
                        updateAnswerAt(index, (current) => ({
                          ...current,
                          selected: option,
                        }))
                      }}
                    />
                    <span>{option}</span>
                  </label>
                ))}

                <label className="flex items-start gap-2 text-sm cursor-pointer">
                  <input
                    type="radio"
                    name={groupName}
                    className="mt-1"
                    checked={row.selected === CUSTOM_VALUE}
                    onChange={() => {
                      updateAnswerAt(index, (current) => ({
                        ...current,
                        selected: CUSTOM_VALUE,
                      }))
                    }}
                  />
                  <span>{ai.askUser.customOptionLabel}</span>
                </label>

                {row.selected === CUSTOM_VALUE ? (
                  <Input
                    value={row.custom}
                    onChange={(event) => {
                      const value = event.target.value
                      updateAnswerAt(index, (current) => ({
                        ...current,
                        selected: CUSTOM_VALUE,
                        custom: value,
                      }))
                    }}
                    placeholder={ai.askUser.customOptionPlaceholder}
                    className="mt-1"
                  />
                ) : null}
              </div>
            </section>
          )
        })}
      </div>

      <div className="flex items-center justify-between gap-2 pt-1">
        <span className="text-xs text-muted-foreground">{ai.askUser.requiredHint}</span>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            className={cn('h-7 px-2.5 text-xs')}
            onClick={() => {
              setAnswersByCallId((prev) => {
                const next = { ...prev }
                delete next[request.callId]
                return next
              })
              onCancel(request.callId)
            }}
          >
            {ai.action.cancel}
          </Button>

          <Button
            type="button"
            size="sm"
            className={cn('h-7 px-2.5 text-xs')}
            disabled={disabled}
            onClick={() => {
              const payload = buildAnswers({ request, states: answers })
              setAnswersByCallId((prev) => {
                const next = { ...prev }
                delete next[request.callId]
                return next
              })
              onSubmit(request.callId, payload)
            }}
          >
            {ai.askUser.submit}
          </Button>
        </div>
      </div>
    </AiApprovalShell>
  )
}
