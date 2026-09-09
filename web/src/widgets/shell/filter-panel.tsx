'use client'

import { useState, type FC, type FormEvent } from 'react'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { cn } from '@shared/lib/cn'
import { serializeSearchParams } from '@shared/lib/search-params'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { Label } from '@shared/ui/label'

const FILTER_ROUTES = ['/search']

const MODE_OPTIONS = [
    { value: '', label: '전체' },
    { value: 'BEAT_7K', label: '7K' },
    { value: 'BEAT_14K', label: '14K' },
    { value: 'BEAT_5K', label: '5K' },
]

const LEVEL_OPTIONS = ['8', '9', '10', '11', '12']

export const FilterPanel: FC = () => {
    const router = useRouter()
    const pathname = usePathname()
    const searchParams = useSearchParams()

    const [keyword, setKeyword] = useState(searchParams.get('q') ?? '')
    const mode = searchParams.get('mode') ?? ''
    const level = searchParams.get('level') ?? ''

    const push = (next: Record<string, string | undefined>) => {
        const merged = serializeSearchParams({
            q: searchParams.get('q') ?? undefined,
            mode,
            level,
            sort: searchParams.get('sort') ?? undefined,
            ...next,
        })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    const submitKeyword = (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()
        push({ q: keyword, page: undefined })
    }

    if (!FILTER_ROUTES.includes(pathname)) return null

    return (
        <aside aria-label='필터' className='bg-sidebar hidden w-80 shrink-0 flex-col gap-3 p-3 lg:flex'>
            <form onSubmit={submitKeyword} className='flex flex-col gap-2'>
                <Label htmlFor='filter-keyword' className='text-2xs text-muted-foreground font-medium tracking-wide uppercase'>
                    검색어
                </Label>
                <Input id='filter-keyword' value={keyword} onChange={(event) => setKeyword(event.target.value)} placeholder='제목 · 아티스트' />
                <Button type='submit' variant='secondary' className='h-8'>
                    적용
                </Button>
            </form>

            <fieldset className='flex flex-col gap-2'>
                <legend className='text-2xs text-muted-foreground font-medium tracking-wide uppercase'>모드</legend>
                <div className='flex flex-wrap gap-1'>
                    {MODE_OPTIONS.map((option) => (
                        <button
                            key={option.label}
                            type='button'
                            aria-pressed={mode === option.value}
                            onClick={() => push({ mode: option.value, page: undefined })}
                            className={cn(
                                'border-border h-7 border px-2 text-xs font-medium',
                                mode === option.value ? 'bg-accent text-accent-foreground' : 'text-muted-foreground',
                            )}>
                            {option.label}
                        </button>
                    ))}
                </div>
            </fieldset>

            <fieldset className='flex flex-col gap-2'>
                <legend className='text-2xs text-muted-foreground font-medium tracking-wide uppercase'>레벨</legend>
                <div className='flex flex-wrap gap-1'>
                    {LEVEL_OPTIONS.map((option) => (
                        <button
                            key={option}
                            type='button'
                            aria-pressed={level === option}
                            onClick={() => push({ level: level === option ? '' : option, page: undefined })}
                            className={cn(
                                'border-border h-7 border px-2 text-xs font-medium tabular-nums',
                                level === option ? 'bg-accent text-accent-foreground' : 'text-muted-foreground',
                            )}>
                            {option}
                        </button>
                    ))}
                </div>
            </fieldset>
        </aside>
    )
}
