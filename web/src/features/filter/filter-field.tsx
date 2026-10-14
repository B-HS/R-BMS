import type { FC, PropsWithChildren } from 'react'

type FilterFieldProps = PropsWithChildren<{
    label: string
    htmlFor?: string
}>

export const FilterField: FC<FilterFieldProps> = ({ label, htmlFor, children }) => (
    <div className='flex min-w-0 items-center gap-2'>
        {htmlFor ? (
            <label htmlFor={htmlFor} className='rbms-micro-label shrink-0'>
                {label}
            </label>
        ) : (
            <span className='rbms-micro-label shrink-0'>{label}</span>
        )}
        {children}
    </div>
)
