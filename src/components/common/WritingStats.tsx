import { BarChart3, Clock } from 'lucide-react'
import { useState } from 'react'
import { InputDialog } from './InputDialog'
import { useTranslation } from '@/hooks/use-translation'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/magic-ui/components'

interface WritingStatsProps {
 words: number
 chars: number
 paragraphs: number
 readingTime: number
 wordGoal: number
 onWordGoalChange?: (newGoal: number | null) => void
 extraActions?: React.ReactNode
}

function WritingStatsSummary(input: {
 words: number
 chars: number
 paragraphs: number
 wordsLabel: string
 charsLabel: string
 paragraphsLabel: string
}) {
 return (
 <div className="grid grid-cols-3 gap-4 mb-4">
 <div className="text-center">
 <div className="stat-value">{input.words}</div>
 <div className="stat-label">{input.wordsLabel}</div>
 </div>
 <div className="text-center">
 <div className="stat-value">{input.chars}</div>
 <div className="stat-label">{input.charsLabel}</div>
 </div>
 <div className="text-center">
 <div className="stat-value">{input.paragraphs}</div>
 <div className="stat-label">{input.paragraphsLabel}</div>
 </div>
 </div>
 )
}

function WritingGoalProgress(input: {
 words: number
 goal: number
 progress: number
 goalLabel: string
 hint: string
 onContextMenu: (e: React.MouseEvent) => void
}) {
 return (
 <Tooltip>
 <TooltipTrigger asChild>
 <div className="mb-3 cursor-context-menu" onContextMenu={input.onContextMenu}>
 <div className="flex items-center justify-between stat-label mb-1">
 <span>{input.goalLabel}</span>
 <span className="font-medium">
 {input.words} / {input.goal}
 </span>
 </div>
 <div className="progress-bar">
 <div className="progress-bar-fill" style={{ width: `${input.progress}%` }} />
 </div>
 </div>
 </TooltipTrigger>
 <TooltipContent variant="success">{input.hint}</TooltipContent>
 </Tooltip>
 )
}

export function WritingStats({ words, chars, paragraphs, readingTime, wordGoal, onWordGoalChange, extraActions }: WritingStatsProps) {
 const progress = Math.min((words / wordGoal) * 100, 100)
 const [showGoalDialog, setShowGoalDialog] = useState(false)
 const { translations } = useTranslation()

 const handleGoalConfirm = (value: string) => {
 const newGoal = parseInt(value, 10)
 if (!isNaN(newGoal) && newGoal > 0 && onWordGoalChange) {
 onWordGoalChange(newGoal)
 }
 }

 return (
 <TooltipProvider>
 <div className="p-4" style={{ borderTop: "1px solid var(--border-color)", background: "var(--bg-white)" }}>
 <div className="flex items-center gap-2 mb-4">
 <BarChart3 className="h-4 w-4" style={{ color: "var(--text-primary-dark)" }} />
 <h3 className="text-sm font-semibold flex-1">{translations.editor.statsTitle}</h3>
 {extraActions}
 </div>

 <WritingStatsSummary
 words={words}
 chars={chars}
 paragraphs={paragraphs}
 wordsLabel={translations.editor.statsWords}
 charsLabel={translations.editor.statsChars}
 paragraphsLabel={translations.editor.statsParagraphs}
 />

 <WritingGoalProgress
 words={words}
 goal={wordGoal}
 progress={progress}
 goalLabel={translations.editor.statsWritingGoal}
 hint={translations.editor.statsRightClickToSetGoal}
 onContextMenu={(e) => {
 e.preventDefault()
 setShowGoalDialog(true)
 }}
 />

 <div className="flex items-center gap-1.5 stat-label">
 <Clock className="h-3.5 w-3.5" />
 <span>
 {translations.editor.statsEstimatedReading} {readingTime} {translations.editor.statsMinutes}
 </span>
 </div>

 {showGoalDialog && (
 <InputDialog
 open={showGoalDialog}
 title={translations.editor.statsSetGoalTitle}
 placeholder={translations.editor.statsSetGoalPlaceholder}
 defaultValue={wordGoal.toString()}
 onClose={() => setShowGoalDialog(false)}
 onConfirm={handleGoalConfirm}
 />
 )}
 </div>
 </TooltipProvider>
 )
}