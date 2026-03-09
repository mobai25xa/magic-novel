import { Zap, Blocks } from 'lucide-react'

interface SkillHeroProps {
  tw: Record<string, string>
}

export function SkillHero({ tw }: SkillHeroProps) {
  return (
    <div className="bento-card span-12 skills-hero">
      <div className="skills-hero-info">
        <h1>
          <Zap size={28} />
          {tw.heroTitle}
        </h1>
        <p>{tw.heroDescription}</p>
      </div>
      <div className="skills-hero-illustration">
        <Blocks size={64} />
      </div>
    </div>
  )
}
