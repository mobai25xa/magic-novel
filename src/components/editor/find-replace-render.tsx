import { FindReplaceMainRow, FindReplaceReplaceRow } from './find-replace-layout'

type Props = {
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
  replaceText: string
  setReplaceText: (value: string) => void
  onReplace: () => void
  onReplaceAll: () => void
}

export function FindReplacePanelView(input: Props) {
  return (
    <div className="find-replace-bar">
      <FindReplaceMainRow
        showReplace={input.showReplace}
        setShowReplace={input.setShowReplace}
        findInputRef={input.findInputRef}
        findText={input.findText}
        setFindText={input.setFindText}
        totalMatches={input.totalMatches}
        currentMatch={input.currentMatch}
        caseSensitive={input.caseSensitive}
        setCaseSensitive={input.setCaseSensitive}
        useRegex={input.useRegex}
        setUseRegex={input.setUseRegex}
        onFindPrevious={input.onFindPrevious}
        onFindNext={input.onFindNext}
        onClose={input.onClose}
      />

      {input.showReplace ? (
        <FindReplaceReplaceRow
          replaceText={input.replaceText}
          setReplaceText={input.setReplaceText}
          findText={input.findText}
          totalMatches={input.totalMatches}
          onReplace={input.onReplace}
          onReplaceAll={input.onReplaceAll}
          onClose={input.onClose}
        />
      ) : null}
    </div>
  )
}
