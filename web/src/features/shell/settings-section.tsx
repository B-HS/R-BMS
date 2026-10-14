import { useId, type FC, type PropsWithChildren, type ReactNode } from 'react'

type SettingsSectionProps = PropsWithChildren<{
    title: string
    description?: string
    action?: ReactNode
}>

export const SettingsSection: FC<SettingsSectionProps> = ({ title, description, action, children }) => {
    const headingId = useId()

    return (
        <section aria-labelledby={headingId} className='border-border bg-card flex flex-col gap-3 border-b p-3'>
            <div className='flex flex-wrap items-start justify-between gap-2'>
                <div className='flex min-w-0 flex-col gap-0.5'>
                    <h2 id={headingId} className='text-sm font-semibold'>
                        {title}
                    </h2>
                    {description && <p className='text-muted-foreground text-xs'>{description}</p>}
                </div>
                {action}
            </div>
            <div className='flex max-w-xl flex-col gap-3'>{children}</div>
        </section>
    )
}
