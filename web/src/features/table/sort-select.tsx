'use client'

import type { FC } from 'react'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'

type SortSelectProps = {
    label: string
    value: string
    options: { value: string; label: string }[]
    onValueChange: (value: string) => void
}

export const SortSelect: FC<SortSelectProps> = ({ label, value, options, onValueChange }) => (
    <Select value={value} onValueChange={onValueChange}>
        <SelectTrigger aria-label={label} size='sm' className='w-56'>
            <SelectValue placeholder={label} />
        </SelectTrigger>
        <SelectContent>
            {options.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                    {option.label}
                </SelectItem>
            ))}
        </SelectContent>
    </Select>
)
