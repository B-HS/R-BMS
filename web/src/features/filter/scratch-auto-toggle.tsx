'use client'

import { useId, type FC } from 'react'
import { SCRATCH_AUTO_TOGGLE_LABEL } from '@shared/constants/scratch-auto'
import { Switch } from '@shared/ui/switch'

type ScratchAutoToggleProps = {
    checked: boolean
    onCheckedChange: (checked: boolean) => void
}

export const ScratchAutoToggle: FC<ScratchAutoToggleProps> = ({ checked, onCheckedChange }) => {
    const id = useId()

    return (
        <div className='flex items-center gap-2'>
            <Switch id={id} size='sm' checked={checked} onCheckedChange={onCheckedChange} />
            <label htmlFor={id} className='cursor-pointer text-xs whitespace-nowrap'>
                {SCRATCH_AUTO_TOGGLE_LABEL}
            </label>
        </div>
    )
}
