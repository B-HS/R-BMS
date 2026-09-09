import { SignupForm } from '@widgets/auth/signup-form'

const SignupPage = () => (
    <div className='flex flex-1 items-center justify-center p-4'>
        <div className='bg-card border-border w-full max-w-sm rounded-lg border p-6 shadow-sm'>
            <SignupForm />
        </div>
    </div>
)

export default SignupPage
