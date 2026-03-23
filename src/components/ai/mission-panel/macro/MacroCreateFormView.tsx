import { Button } from '@/magic-ui/components'

export type MacroCreateFormState = {
  objective: string
  workflowKind: 'book' | 'volume'
  tokenBudget: 'small' | 'medium' | 'large'
  strictReview: boolean
  autoFixOnBlock: boolean
  targetsText: string
}

type MacroCreateObjectiveFieldProps = {
  value: string
  disabled: boolean
  onChange: (value: string) => void
}

function MacroCreateObjectiveField({ value, disabled, onChange }: MacroCreateObjectiveFieldProps) {
  return (
    <div className="space-y-1">
      <div className="text-[11px] font-medium text-secondary-foreground">Objective</div>
      <input
        className="w-full rounded border border-border bg-background px-2 py-1 text-xs"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="e.g. Advance chapters"
        disabled={disabled}
      />
    </div>
  )
}

type MacroCreateKindBudgetFieldsProps = {
  workflowKind: 'book' | 'volume'
  tokenBudget: 'small' | 'medium' | 'large'
  disabled: boolean
  onWorkflowKindChange: (kind: 'book' | 'volume') => void
  onTokenBudgetChange: (budget: 'small' | 'medium' | 'large') => void
}

function MacroCreateKindBudgetFields({
  workflowKind,
  tokenBudget,
  disabled,
  onWorkflowKindChange,
  onTokenBudgetChange,
}: MacroCreateKindBudgetFieldsProps) {
  return (
    <div className="grid grid-cols-2 gap-2">
      <div className="space-y-1">
        <div className="text-[11px] font-medium text-secondary-foreground">Kind</div>
        <select
          className="w-full rounded border border-border bg-background px-2 py-1 text-xs"
          value={workflowKind}
          onChange={(e) => onWorkflowKindChange(e.target.value === 'volume' ? 'volume' : 'book')}
          disabled={disabled}
        >
          <option value="book">book</option>
          <option value="volume">volume</option>
        </select>
      </div>

      <div className="space-y-1">
        <div className="text-[11px] font-medium text-secondary-foreground">Token budget</div>
        <select
          className="w-full rounded border border-border bg-background px-2 py-1 text-xs"
          value={tokenBudget}
          onChange={(e) => {
            const v = e.target.value
            onTokenBudgetChange(v === 'small' ? 'small' : v === 'large' ? 'large' : 'medium')
          }}
          disabled={disabled}
        >
          <option value="small">small</option>
          <option value="medium">medium</option>
          <option value="large">large</option>
        </select>
      </div>
    </div>
  )
}

type MacroCreateFlagFieldsProps = {
  strictReview: boolean
  autoFixOnBlock: boolean
  disabled: boolean
  onStrictReviewChange: (value: boolean) => void
  onAutoFixOnBlockChange: (value: boolean) => void
}

function MacroCreateFlagFields({
  strictReview,
  autoFixOnBlock,
  disabled,
  onStrictReviewChange,
  onAutoFixOnBlockChange,
}: MacroCreateFlagFieldsProps) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <label className="inline-flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={strictReview}
          onChange={(e) => onStrictReviewChange(e.target.checked)}
          disabled={disabled}
        />
        strict_review
      </label>
      <label className="inline-flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={autoFixOnBlock}
          onChange={(e) => onAutoFixOnBlockChange(e.target.checked)}
          disabled={disabled}
        />
        auto_fix_on_block
      </label>
    </div>
  )
}

type MacroCreateTargetsFieldProps = {
  value: string
  disabled: boolean
  onChange: (value: string) => void
}

function MacroCreateTargetsField({ value, disabled, onChange }: MacroCreateTargetsFieldProps) {
  return (
    <div className="space-y-1">
      <div className="text-[11px] font-medium text-secondary-foreground">chapter_targets</div>
      <textarea
        className="w-full min-h-28 rounded border border-border bg-background px-2 py-1 font-mono text-[11px]"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      />
      <div className="text-[11px] text-muted-foreground">
        write_path must be manuscripts-relative chapter JSON (e.g. <span className="font-mono">vol_1/ch_001.json</span>). You can also paste one write_path per line.
      </div>
    </div>
  )
}

type MacroCreateSubmitButtonProps = {
  creating: boolean
  disabled: boolean
  onClick: () => void
}

function MacroCreateSubmitButton({ creating, disabled, onClick }: MacroCreateSubmitButtonProps) {
  return (
    <Button variant="outline" size="sm" className="text-xs w-full" onClick={onClick} disabled={disabled}>
      {creating ? 'Creating…' : 'Create'}
    </Button>
  )
}

export function MacroCreateFormView(input: {
  open: boolean
  onOpenChange: (open: boolean) => void
  creating: boolean
  loading: boolean
  error: string | null
  form: MacroCreateFormState
  setForm: (next: (prev: MacroCreateFormState) => MacroCreateFormState) => void
  onCreate: () => void
}) {
  const disabled = input.creating || input.loading

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={input.open}
      onToggle={(event) => input.onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        Create Macro Workflow
      </summary>

      {input.error ? (
        <p className="mt-2 text-xs text-muted-foreground">Macro create failed: {input.error}</p>
      ) : null}

      <div className="mt-2 space-y-2 text-xs">
        <MacroCreateObjectiveField
          value={input.form.objective}
          disabled={disabled}
          onChange={(objective) => input.setForm((prev) => ({ ...prev, objective }))}
        />

        <MacroCreateKindBudgetFields
          workflowKind={input.form.workflowKind}
          tokenBudget={input.form.tokenBudget}
          disabled={disabled}
          onWorkflowKindChange={(workflowKind) => input.setForm((prev) => ({ ...prev, workflowKind }))}
          onTokenBudgetChange={(tokenBudget) => input.setForm((prev) => ({ ...prev, tokenBudget }))}
        />

        <MacroCreateFlagFields
          strictReview={input.form.strictReview}
          autoFixOnBlock={input.form.autoFixOnBlock}
          disabled={disabled}
          onStrictReviewChange={(strictReview) => input.setForm((prev) => ({ ...prev, strictReview }))}
          onAutoFixOnBlockChange={(autoFixOnBlock) => input.setForm((prev) => ({ ...prev, autoFixOnBlock }))}
        />

        <MacroCreateTargetsField
          value={input.form.targetsText}
          disabled={disabled}
          onChange={(targetsText) => input.setForm((prev) => ({ ...prev, targetsText }))}
        />

        <MacroCreateSubmitButton creating={input.creating} disabled={disabled} onClick={input.onCreate} />
      </div>
    </details>
  )
}

