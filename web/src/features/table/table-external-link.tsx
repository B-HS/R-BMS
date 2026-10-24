import type { FC } from 'react'
import { ExternalLink } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'

type TableExternalLinkProps = {
    href: string
    label: string
    appearance: 'button' | 'text'
}

export const TableExternalLink: FC<TableExternalLinkProps> = ({ href, label, appearance }) => {
    const t = useTranslations('pages.table')
    const content = (
        <a
            href={href}
            target='_blank'
            rel='noopener noreferrer'
            className={
                appearance === 'text' ? 'text-foreground inline-flex items-center gap-0.5 underline underline-offset-2 hover:no-underline' : undefined
            }>
            {label}
            <ExternalLink aria-hidden className={appearance === 'text' ? 'size-3' : undefined} />
            <span className='sr-only'>({t('externalLink')})</span>
        </a>
    )

    if (appearance === 'text') return content

    return (
        <Button asChild variant='outline' size='sm'>
            {content}
        </Button>
    )
}
