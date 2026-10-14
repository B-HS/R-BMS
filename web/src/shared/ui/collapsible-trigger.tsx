'use client'

import type { ComponentProps, FC } from 'react'
import { Collapsible as CollapsiblePrimitive } from 'radix-ui'

export const CollapsibleTrigger: FC<ComponentProps<typeof CollapsiblePrimitive.CollapsibleTrigger>> = (props) => (
    <CollapsiblePrimitive.CollapsibleTrigger data-slot='collapsible-trigger' {...props} />
)
