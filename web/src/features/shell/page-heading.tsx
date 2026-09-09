import type { FC, PropsWithChildren } from 'react'

export const PageHeading: FC<PropsWithChildren> = ({ children }) => (
    <h1 className='text-foreground px-4 pt-4 pb-1 text-lg font-semibold tracking-tight'>{children}</h1>
)
