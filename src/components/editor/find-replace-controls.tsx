import { CaseSensitive, ChevronDown, ChevronUp, Regex, X } from 'lucide-react'
import { useTranslation } from '@/hooks/use-translation'

type FindReplaceControlsProps = {
  showReplace: boolean
  setShowReplace: (value: boolean) => void
  findText: string
  totalMatches: number
  currentMatch: number
  caseSensitive: boolean
  setCaseSensitive: (value: boolean) => void
  useRegex: boolean
  setUseRegex: (value: boolean) => void
  onFindPrevious: () => void
  onFindNext: () => void
  onClose: () => void
}

export function FindReplaceControls(props: FindReplaceControlsProps) {
  const {
    showReplace,
    setShowReplace,
    findText,
    totalMatches,
    currentMatch,
    caseSensitive,
    setCaseSensitive,
    useRegex,
    setUseRegex,
    onFindPrevious,
    onFindNext,
    onClose,
  } = props
  const { translations } = useTranslation()
  const fr = translations.findReplace

  return (
    <>
      <button
        onClick={() => setShowReplace(!showReplace)}
        className="find-replace-icon-btn"
        title={showReplace ? fr.hideReplace : fr.showReplace}
      >
        <ChevronDown className={`h-4 w-4 transition-transform ${showReplace ? '' : '-rotate-90'}`} />
      </button>

      {findText ? (
        <span className="text-xs whitespace-nowrap" style={{ color: "var(--text-muted-foreground)" }}>
          {totalMatches > 0 ? `${currentMatch}/${totalMatches}` : fr.noResults}
        </span>
      ) : null}

      <button
        onClick={() => setCaseSensitive(!caseSensitive)}
        className={`find-replace-icon-btn ${caseSensitive ? 'find-replace-toggle-active' : ''}`}
        title={fr.caseSensitive}
      >
        <CaseSensitive className="h-4 w-4" />
      </button>

      <button
        onClick={() => setUseRegex(!useRegex)}
        className={`find-replace-icon-btn ${useRegex ? 'find-replace-toggle-active' : ''}`}
        title={fr.regex}
      >
        <Regex className="h-4 w-4" />
      </button>

      <button
        onClick={onFindPrevious}
        disabled={!findText || totalMatches === 0}
        className="find-replace-icon-btn"
        title={fr.prevMatch}
      >
        <ChevronUp className="h-4 w-4" />
      </button>
      <button
        onClick={onFindNext}
        disabled={!findText || totalMatches === 0}
        className="find-replace-icon-btn"
        title={fr.nextMatch}
      >
        <ChevronDown className="h-4 w-4" />
      </button>

      <button
        onClick={onClose}
        className="find-replace-icon-btn ml-auto"
        title={fr.close}
      >
        <X className="h-4 w-4" />
      </button>
    </>
  )
}