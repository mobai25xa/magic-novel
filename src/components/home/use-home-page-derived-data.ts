import { useMemo } from 'react'

type ProjectListItem = {
  path: string
  name: string
  author: string
  lastOpenedAt: number
  coverImage?: string
}

type RecycledItem = {
  id: string
  path: string
  name: string
  author: string
  deletedAt: number
  coverImage?: string
}

type Stats = {
  genresByProjectPath: Record<string, string[]>
}

type Input = {
  typeFilter: string | null
  projectList: ProjectListItem[]
  recycledProjects: RecycledItem[]
  genresByProjectPath: Stats['genresByProjectPath']
}

function buildFilteredProjects(input: {
  typeFilter: Input['typeFilter']
  projectList: Input['projectList']
  genresByProjectPath: Input['genresByProjectPath']
}) {
  return input.projectList.filter((project) => {
    if (!input.typeFilter) return true
    const genres = input.genresByProjectPath[project.path] || []
    return genres.includes(input.typeFilter)
  })
}

export function useHomePageDerivedData(input: Input) {
  const filteredProjects = useMemo(
    () =>
      buildFilteredProjects({
        typeFilter: input.typeFilter,
        projectList: input.projectList,
        genresByProjectPath: input.genresByProjectPath,
      }),
    [input.typeFilter, input.projectList, input.genresByProjectPath],
  )

  return {
    filteredProjects,
    recycledProjects: input.recycledProjects,
  }
}
