'use client'

import type { ComponentProps, FC } from 'react'
import { Collapsible as CollapsiblePrimitive } from 'radix-ui'

export const Collapsible: FC<ComponentProps<typeof CollapsiblePrimitive.Root>> = (props) => (
    <CollapsiblePrimitive.Root data-slot='collapsible' {...props} />
)
