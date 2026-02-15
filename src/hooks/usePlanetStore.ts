import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  MyPlanet,
  MyArticle,
  Draft,
  PlanetStoreSnapshot,
} from '../types/planet'

export function usePlanetStore() {
  const [myPlanets, setMyPlanets] = useState<MyPlanet[]>([])
  const [loading, setLoading] = useState(true)

  // 初始加载
  useEffect(() => {
    invoke<PlanetStoreSnapshot>('planet_get_state')
      .then((state) => {
        setMyPlanets(state.my_planets)
      })
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [])

  // 监听状态变化事件
  useEffect(() => {
    const unlisten = listen<PlanetStoreSnapshot>(
      'planet:state-changed',
      (event) => {
        setMyPlanets(event.payload.my_planets)
      }
    )
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // 创建 Planet
  const createPlanet = useCallback(
    async (name: string, about: string, templateName?: string) => {
      return invoke<MyPlanet>('planet_create', {
        request: {
          name,
          about,
          template_name: templateName || 'Plain',
        },
      })
    },
    []
  )

  // 删除 Planet
  const deletePlanet = useCallback(async (planetId: string) => {
    return invoke<void>('planet_delete', { planetId })
  }, [])

  // 更新 Planet
  const updatePlanet = useCallback(
    async (planetId: string, updates: Partial<MyPlanet>) => {
      return invoke<MyPlanet>('planet_update', {
        planetId,
        request: updates,
      })
    },
    []
  )

  return {
    myPlanets,
    loading,
    createPlanet,
    deletePlanet,
    updatePlanet,
  }
}

export function useArticles(planetId: string | null) {
  const [articles, setArticles] = useState<MyArticle[]>([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!planetId) {
      setArticles([])
      return
    }
    setLoading(true)
    invoke<MyArticle[]>('article_list', { planetId })
      .then(setArticles)
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [planetId])

  const createArticle = useCallback(
    async (title: string, content: string) => {
      if (!planetId) throw new Error('No planet selected')
      const article = await invoke<MyArticle>('article_create', {
        request: { planet_id: planetId, title, content },
      })
      setArticles((prev) => [article, ...prev])
      return article
    },
    [planetId]
  )

  const deleteArticle = useCallback(
    async (articleId: string) => {
      if (!planetId) throw new Error('No planet selected')
      await invoke<void>('article_delete', { planetId, articleId })
      setArticles((prev) => prev.filter((a) => a.id !== articleId))
    },
    [planetId]
  )

  const updateArticle = useCallback(
    async (articleId: string, title?: string, content?: string) => {
      if (!planetId) throw new Error('No planet selected')
      const updated = await invoke<MyArticle>('article_update', {
        planetId,
        articleId,
        request: { title, content },
      })
      setArticles((prev) =>
        prev.map((a) => (a.id === articleId ? updated : a))
      )
      return updated
    },
    [planetId]
  )

  return { articles, loading, createArticle, deleteArticle, updateArticle }
}