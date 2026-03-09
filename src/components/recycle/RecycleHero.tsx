import { ArchiveRestore, PackageOpen } from 'lucide-react'

interface RecycleHeroProps {
  tw: Record<string, string>
}

export function RecycleHero({ tw }: RecycleHeroProps) {
  return (
    <div className="bento-card span-12 recycle-hero">
      <div className="recycle-hero-info">
        <h1>
          <ArchiveRestore size={28} />
          {tw.heroTitle}
        </h1>
        <p>{tw.heroDescription}</p>
      </div>
      <div className="recycle-hero-illustration">
        <PackageOpen size={64} />
      </div>
    </div>
  )
}
