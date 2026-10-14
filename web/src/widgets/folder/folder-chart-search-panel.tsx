'use client'

import { useEffect, useId, useState, type FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Search } from 'lucide-react'
import { ChartSearchResultRow } from '@features/folder/chart-search-result-row'
import { chartSearchQueryOptions } from '@entities/chart.query'
import { isFolderMd5, type FolderItem, type FolderTableIndex } from '@entities/folder.type'
import { formatCount } from '@shared/lib/format'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'

const SEARCH_DEBOUNCE_MS = 300
const SEARCH_MIN_LENGTH = 2
const MD5_PREFIX_MIN_LENGTH = 6
const TABLE_RESULT_LIMIT = 50
const SERVER_RESULT_LIMIT = 30
const HEX_PATTERN = /^[0-9a-f]+$/

type SearchResult = Pick<FolderItem, 'md5' | 'sha256' | 'title' | 'artist'> & { isKnownToServer: boolean }

type FolderChartSearchPanelProps = {
    tables: FolderTableIndex[]
    levelLabelByMd5: ReadonlyMap<string, string>
    presentMd5s: ReadonlySet<string>
    isFull: boolean
    onAdd: (item: SearchResult) => void
    autoFocus?: boolean
    isLabelVisible?: boolean
    className?: string
}

const normaliseSearchText = (value: string) => value.normalize('NFKC').toLowerCase()

export const FolderChartSearchPanel: FC<FolderChartSearchPanelProps> = ({
    tables,
    levelLabelByMd5,
    presentMd5s,
    isFull,
    onAdd,
    autoFocus = false,
    isLabelVisible = true,
    className,
}) => {
    const [query, setQuery] = useState('')
    const [debouncedQuery, setDebouncedQuery] = useState('')
    const [lastAddedTitle, setLastAddedTitle] = useState('')

    const inputId = useId()
    const needle = normaliseSearchText(debouncedQuery)
    const isSearchable = needle.length >= SEARCH_MIN_LENGTH
    const isMd5Prefix = needle.length >= MD5_PREFIX_MIN_LENGTH && HEX_PATTERN.test(needle)

    const tableMatches = isSearchable
        ? tables
              .flatMap((table) => table.levels.flatMap((group) => group.entries))
              .filter(
                  (entry) =>
                      (isMd5Prefix && entry.md5.startsWith(needle)) ||
                      normaliseSearchText(entry.title).includes(needle) ||
                      normaliseSearchText(entry.artist).includes(needle),
              )
        : []
    const tableHits = tableMatches.slice(0, TABLE_RESULT_LIMIT)
    const isTruncated = tableMatches.length > TABLE_RESULT_LIMIT

    const handleAdd = (result: SearchResult) => {
        onAdd(result)
        setLastAddedTitle(result.title || result.md5)
    }

    const server = useQuery({
        ...chartSearchQueryOptions({ q: debouncedQuery, page: 1, limit: SERVER_RESULT_LIMIT }),
        enabled: isSearchable,
        meta: { suppressErrorToast: true },
    })

    useEffect(() => {
        const timer = setTimeout(() => setDebouncedQuery(query.trim()), SEARCH_DEBOUNCE_MS)
        return () => clearTimeout(timer)
    }, [query])

    const serverRows = [...new Map((server.data?.data ?? []).filter((row) => isFolderMd5(row.md5)).map((row) => [row.md5, row])).values()]
    const serverByMd5 = new Map(serverRows.map((row) => [row.md5, row]))
    const tableMd5s = new Set(tableHits.map((entry) => entry.md5))
    const results: SearchResult[] = [
        ...tableHits.map((entry) => ({ ...entry, sha256: serverByMd5.get(entry.md5)?.sha256 ?? '', isKnownToServer: serverByMd5.has(entry.md5) })),
        ...serverRows
            .filter((row) => !tableMd5s.has(row.md5))
            .map((row) => ({ md5: row.md5, sha256: row.sha256, title: row.title, artist: row.artist, isKnownToServer: true })),
    ]
    const isServerSearching = isSearchable && server.isFetching
    const isWaiting = query.trim() !== debouncedQuery

    const statusText = (() => {
        if (!isSearchable) return '두 글자 이상 입력하세요. 곡명, 아티스트, md5 앞 6자로 찾습니다.'
        if (results.length === 0 && (isServerSearching || isWaiting)) return '찾는 중'
        if (results.length === 0) return '조건에 맞는 곡이 없습니다.'
        return `${formatCount(results.length)}곡${isTruncated ? ' 이상 · 검색어를 더 좁혀 보세요' : ''}${isServerSearching ? ' · IR 검색 중' : ''}`
    })()

    return (
        <section aria-labelledby={`${inputId}-label`} className={cn('flex min-h-0 min-w-0 flex-col', className)}>
            <div className='border-border flex flex-col gap-2 border-b p-3'>
                <label id={`${inputId}-label`} htmlFor={inputId} className={isLabelVisible ? 'rbms-micro-label' : 'sr-only'}>
                    곡 추가
                </label>
                <div className='relative'>
                    <Search className='text-muted-foreground pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2' aria-hidden />
                    <Input
                        id={inputId}
                        type='search'
                        autoComplete='off'
                        enterKeyHint='search'
                        autoFocus={autoFocus}
                        placeholder='곡명 · 아티스트 · md5'
                        className='bg-background pl-7'
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                    />
                </div>
                <p aria-live='polite' className='text-muted-foreground text-xs tabular-nums'>
                    {statusText}
                    {lastAddedTitle && <span className='sr-only'>. {lastAddedTitle} 을 목록 끝에 넣었습니다.</span>}
                </p>
                {isFull && <p className='text-destructive text-xs'>폴더가 가득 찼습니다. 곡을 빼야 더 넣을 수 있습니다.</p>}
                {isSearchable && server.isError && (
                    <p className='text-muted-foreground flex items-center gap-1 text-xs'>
                        IR 검색을 불러오지 못해 난이도표 결과만 보입니다.
                        <Button variant='link' size='xs' className='h-auto px-0' onClick={() => void server.refetch()}>
                            다시 시도
                        </Button>
                    </p>
                )}
            </div>
            {results.length > 0 && (
                <ul aria-label='검색 결과' className='rbms-list-scroll min-h-0 flex-1 overflow-y-auto'>
                    {results.map((result) => (
                        <ChartSearchResultRow
                            key={result.md5}
                            md5={result.md5}
                            title={result.title}
                            artist={result.artist}
                            levelLabel={levelLabelByMd5.get(result.md5)}
                            isKnownToServer={result.isKnownToServer}
                            isAdded={presentMd5s.has(result.md5)}
                            isAddDisabled={isFull}
                            onAdd={() => handleAdd(result)}
                        />
                    ))}
                </ul>
            )}
        </section>
    )
}
