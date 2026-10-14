import type { FC, PropsWithChildren } from 'react'

type AuthCardProps = PropsWithChildren<{
    title: string
    description: string
}>

export const AuthCard: FC<AuthCardProps> = ({ title, description, children }) => (
    <div className='flex flex-1 items-start justify-center p-3 md:items-center'>
        <section className='border-border bg-card mx-auto flex w-full max-w-sm flex-col gap-4 border p-6'>
            <div className='flex flex-col gap-1'>
                <h2 className='text-base font-semibold tracking-tight'>{title}</h2>
                <p className='text-muted-foreground text-xs'>{description}</p>
            </div>
            {children}
        </section>
    </div>
)
