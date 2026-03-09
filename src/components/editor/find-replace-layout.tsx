import { Replace, Search } from 'lucide-react'

import { Input } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

import { FindReplaceControls } from './find-replace-controls'

type MainRowProps = {
  showReplace: boolean
  setShowReplace: (value: boolean) => void
  findInputRef: React.RefObject<HTMLInputElement | null>
  findText: string
  setFindText: (value: string) => void
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

export function FindReplaceMainRow(props: MainRowProps) {
  const {
    showReplace,
    setShowReplace,
    findInputRef,
    findText,
    setFindText,
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
    <div className="flex items-center gap-2 px-3 py-2">
      <div className="relative flex-1 max-w-xs">
        <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 pointer-events-none" style={{ color: "var(--text-muted-foreground)" }} />
        <Input
          ref={findInputRef}
          type="text"
          placeholder={fr.searchPlaceholder}
          value={findText}
          onChange={(e) => setFindText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              if (e.shiftKey) {
                onFindPrevious()
              } else {
                onFindNext()
              }
            }
            if (e.key === 'Escape') {
              onClose()
            }
          }}
          className="h-7 pl-7"
          autoFocus
        />
      </div>

      <FindReplaceControls
        showReplace={showReplace}
        setShowReplace={setShowReplace}
        findText={findText}
        totalMatches={totalMatches}
        currentMatch={currentMatch}
        caseSensitive={caseSensitive}
        setCaseSensitive={setCaseSensitive}
        useRegex={useRegex}
        setUseRegex={setUseRegex}
        onFindPrevious={onFindPrevious}
        onFindNext={onFindNext}
        onClose={onClose}
      />
    </div>
  )
}

type ReplaceRowProps = {
  replaceText: string
  setReplaceText: (value: string) => void
  findText: string
  totalMatches: number
  onReplace: () => void
  onReplaceAll: () => void
  onClose: () => void
}

export function FindReplaceReplaceRow(props: ReplaceRowProps) {
  const { replaceText, setReplaceText, findText, totalMatches, onReplace, onReplaceAll, onClose } = props
  const { translations } = useTranslation()
  const fr = translations.findReplace

  return (
    <div className="flex items-center gap-2 px-3 pb-2 pl-[52px]">
      <div className="relative flex-1 max-w-xs">
        <Replace className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 pointer-events-none" style={{ color: "var(--text-muted-foreground)" }} />
        <Input
          type="text"
          placeholder={fr.replacePlaceholder}
          value={replaceText}
          onChange={(e) => setReplaceText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              onReplace()
            }
            if (e.key === 'Escape') {
              onClose()
            }
          }}
          className="h-7 pl-7"
        />
      </div>

      <button
        onClick={onReplace}
        disabled={!findText || totalMatches === 0}
        className="find-replace-icon-btn h-7 px-3 text-xs whitespace-nowrap"
      >
        {fr.replaceOne}
      </button>
      <button
        onClick={onReplaceAll}
        disabled={!findText || totalMatches === 0}
        className="find-replace-icon-btn h-7 px-3 text-xs whitespace-nowrap"
      >
        {fr.replaceAll}
      </button>
    </div>
  )
}
