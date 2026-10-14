import type { FC } from 'react'
import { ExternalLink } from 'lucide-react'
import { Button } from '@shared/ui/button'

type TableExternalLinkProps = {
    href: string
    label: string
    appearance: 'button' | 'text'
}

export const TableExternalLink: FC<TableExternalLinkProps> = ({ href, label, appearance }) => {
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
            <span className='sr-only'>(외부 사이트, 새 탭)</span>
        </a>
    )

    if (appearance === 'text') return content

    return (
        <Button asChild variant='outline' size='sm'>
            {content}
        </Button>
    )
}
