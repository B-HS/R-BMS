import type { FC, PropsWithChildren } from 'react'
import { Badge } from '@shared/ui/badge'

export const MetaBadge: FC<PropsWithChildren> = ({ children }) => <Badge variant='secondary'>{children}</Badge>
