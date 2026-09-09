import type { FC, PropsWithChildren } from 'react'

export const PanelFootnote: FC<PropsWithChildren> = ({ children }) => <p className='text-muted-foreground text-xs'>{children}</p>
