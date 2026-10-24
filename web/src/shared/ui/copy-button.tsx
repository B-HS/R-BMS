'use client'

import { useState, type ComponentProps, type FC } from 'react'
import { Check, Copy } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { COPY_FEEDBACK_DURATION_MS } from '@shared/constants/ui'
import { Button } from '@shared/ui/button'

type CopyButtonProps = Omit<ComponentProps<typeof Button>, 'onClick' | 'children'> & {
    value: string
    label: string
    copiedMessage?: string
    showLabel?: boolean
}

export const CopyButton: FC<CopyButtonProps> = ({ value, label, copiedMessage, showLabel = false, variant = 'outline', size, ...props }) => {
    const t = useTranslations('pages.copyButton')
    const [isCopied, setIsCopied] = useState(false)

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(value)
            setIsCopied(true)
            toast.success(copiedMessage ?? t('copied'))
            window.setTimeout(() => setIsCopied(false), COPY_FEEDBACK_DURATION_MS)
        } catch {
            toast.error(t('failed'))
        }
    }

    return (
        <Button type='button' variant={variant} size={size ?? (showLabel ? 'sm' : 'icon-sm')} aria-label={label} onClick={handleCopy} {...props}>
            {isCopied ? <Check aria-hidden /> : <Copy aria-hidden />}
            {showLabel && <span>{label}</span>}
        </Button>
    )
}
