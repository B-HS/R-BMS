'use client'

import type { ComponentProps, FC } from 'react'
import { Collapsible as CollapsiblePrimitive } from 'radix-ui'

export const CollapsibleContent: FC<ComponentProps<typeof CollapsiblePrimitive.CollapsibleContent>> = (props) => (
    <CollapsiblePrimitive.CollapsibleContent data-slot='collapsible-content' {...props} />
)
