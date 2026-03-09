import { z } from 'zod'

export const ProjectSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  createdAt: z.string(),
  updatedAt: z.string(),
})

export const ChapterSchema = z.object({
  id: z.string(),
  title: z.string(),
  volumeId: z.string(),
  order: z.number(),
  wordCount: z.number(),
  status: z.string(),
})

export const VolumeSchema = z.object({
  id: z.string(),
  title: z.string(),
  order: z.number(),
})

export type Project = z.infer<typeof ProjectSchema>
export type Chapter = z.infer<typeof ChapterSchema>
export type Volume = z.infer<typeof VolumeSchema>
