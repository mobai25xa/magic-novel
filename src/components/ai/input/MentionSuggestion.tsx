import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useState,
} from 'react'
import { BookOpen, FileText, MapPin, Package, User } from 'lucide-react'

import { AiFlyoutShell } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

import { groupMentionItems, type MentionItem } from './mention-data'

export type MentionSuggestionRef = {
  onKeyDown: (props: { event: KeyboardEvent }) => boolean
}

type MentionSuggestionProps = {
  items: MentionItem[]
  command: (item: MentionItem) => void
}

const GROUP_ICONS: Record<string, typeof FileText> = {
  chapter: FileText,
  volume: BookOpen,
  character: User,
  location: MapPin,
  asset: Package,
}

export const MentionSuggestion = forwardRef<MentionSuggestionRef, MentionSuggestionProps>(
  (props, ref) => {
    const { items, command } = props
    const { translations } = useTranslation()
    const labels = translations.aiChat
    const [selectedIndex, setSelectedIndex] = useState(0)

    const GROUP_LABELS: Record<string, string> = {
      chapter: labels.chapter,
      volume: labels.volume,
      character: labels.character,
      location: labels.location,
      asset: labels.asset,
    }

    const grouped = groupMentionItems(items)
    const flatItems = grouped.flatMap((g) => g.items)

    useEffect(() => {
      setSelectedIndex(0)
    }, [items])

    const selectItem = useCallback(
      (index: number) => {
        const item = flatItems[index]
        if (item) {
          command(item)
        }
      },
      [flatItems, command],
    )

    useImperativeHandle(ref, () => ({
      onKeyDown: ({ event }) => {
        if (event.key === 'ArrowUp') {
          setSelectedIndex((prev) => (prev + flatItems.length - 1) % flatItems.length)
          return true
        }

        if (event.key === 'ArrowDown') {
          setSelectedIndex((prev) => (prev + 1) % flatItems.length)
          return true
        }

        if (event.key === 'Enter' || event.key === 'Tab') {
          selectItem(selectedIndex)
          return true
        }

        if (event.key === 'Escape') {
          return true
        }

        return false
      },
    }))

    if (flatItems.length === 0) {
      return (
        <AiFlyoutShell className="p-3">
          <span className="text-xs text-muted-foreground">{labels.noMatch}</span>
        </AiFlyoutShell>
      )
    }

    let flatIndex = 0

    return (
      <AiFlyoutShell className="overflow-hidden max-h-[280px] overflow-y-auto min-w-[200px]" role="listbox">
        {grouped.map((group) => {
          const Icon = GROUP_ICONS[group.group] ?? Package

          return (
            <div key={group.group}>
              <div className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium text-secondary-foreground bg-secondary-30 border-b border-b-border">
                <Icon className="h-3 w-3" />
                <span>{GROUP_LABELS[group.group] ?? group.group}</span>
              </div>
              {group.items.map((item) => {
                const currentIndex = flatIndex++
                const isSelected = currentIndex === selectedIndex

                return (
                  <button
                    key={item.id}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    className={`w-full text-left px-3 py-1.5 text-sm flex items-center gap-2 transition-colors ${
                      isSelected ? 'active-bg text-foreground' : 'hover-bg-50'
                    }`}
                    onClick={() => selectItem(currentIndex)}
                    onMouseEnter={() => setSelectedIndex(currentIndex)}
                  >
                    <span className="truncate">{item.label}</span>
                  </button>
                )
              })}
            </div>
          )
        })}
      </AiFlyoutShell>
    )
  },
)

MentionSuggestion.displayName = 'MentionSuggestion'
