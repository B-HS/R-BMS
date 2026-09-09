import { Suspense } from 'react'
import { LoginForm } from '@widgets/auth/login-form'

const LoginPage = () => (
    <div className='flex flex-1 items-center justify-center p-4'>
        <div className='bg-card border-border w-full max-w-sm rounded-lg border p-6 shadow-sm'>
            <Suspense fallback={null}>
                <LoginForm />
            </Suspense>
        </div>
    </div>
)

export default LoginPage
