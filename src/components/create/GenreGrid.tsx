import { useRef, useCallback } from 'react'
import type { GenreOption } from './types'

interface GenreGridProps {
  genres: GenreOption[]
  selected: string | null
  onSelect: (id: string | null) => void
}

export function GenreGrid({ genres, selected, onSelect }: GenreGridProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent, index: number) => {
      let next = index
      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
        e.preventDefault()
        next = (index + 1) % genres.length
      } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault()
        next = (index - 1 + genres.length) % genres.length
      } else {
        return
      }
      const buttons = containerRef.current?.querySelectorAll<HTMLButtonElement>('button')
      buttons?.[next]?.focus()
      onSelect(genres[next].id)
    },
    [genres, onSelect],
  )

  return (
    <div className="genre-grid" role="radiogroup" ref={containerRef}>
      {genres.map((genre, i) => {
        const isSelected = selected === genre.id
        const Icon = genre.icon
        return (
          <button
            key={genre.id}
            type="button"
            className={`genre-card${isSelected ? ' selected' : ''}`}
            role="radio"
            aria-checked={isSelected}
            aria-label={genre.name}
            tabIndex={isSelected || (!selected && i === 0) ? 0 : -1}
            onClick={() => onSelect(isSelected ? null : genre.id)}
            onKeyDown={(e) => handleKeyDown(e, i)}
          >
            <div className="genre-icon">
              <Icon size={24} />
            </div>
            <span className="genre-name">{genre.name}</span>
          </button>
        )
      })}
    </div>
  )
}
