const PublicLayout = ({ children }: LayoutProps<'/'>) => (
    <div data-surface='public' className='bg-background text-foreground flex min-h-dvh flex-col'>
        {children}
    </div>
)

export default PublicLayout
