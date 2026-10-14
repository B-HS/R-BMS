'use client'

import { useState, type ComponentProps, type FC } from 'react'
import { Check, Copy } from 'lucide-react'
import { toast } from 'sonner'
import { COPY_FEEDBACK_DURATION_MS } from '@shared/constants/ui'
import { Button } from '@shared/ui/button'

type CopyButtonProps = Omit<ComponentProps<typeof Button>, 'onClick' | 'children'> & {
    value: string
    label: string
    copiedMessage?: string
    showLabel?: boolean
}

export const CopyButton: FC<CopyButtonProps> = ({
    value,
    label,
    copiedMessage = '복사했습니다.',
    showLabel = false,
    variant = 'outline',
    size,
    ...props
}) => {
    const [isCopied, setIsCopied] = useState(false)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(value)
            setIsCopied(true)
            toast.success(copiedMessage)
            window.setTimeout(() => setIsCopied(false), COPY_FEEDBACK_DURATION_MS)
        } catch {
            toast.error('복사하지 못했습니다. 직접 선택해 복사하세요.')
        }
    }

    return (
        <Button type='button' variant={variant} size={size ?? (showLabel ? 'sm' : 'icon-sm')} aria-label={label} onClick={handleCopy} {...props}>
            {isCopied ? <Check aria-hidden /> : <Copy aria-hidden />}
            {showLabel && <span>{label}</span>}
        </Button>
    )
}
