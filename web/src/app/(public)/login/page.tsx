import { buildPageMetadata } from '@shared/lib/metadata'
import { AuthCard } from '@widgets/auth/auth-card'
import { LoginForm } from '@widgets/auth/login-form'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '로그인', description: 'R-BMS 계정으로 로그인합니다.', path: '/login' })

const LoginPage = () => (
    <ShellPage title='로그인' contentClassName='gap-0'>
        <AuthCard title='로그인' description='가입할 때 정한 아이디와 비밀번호로 로그인합니다.'>
            <LoginForm />
        </AuthCard>
    </ShellPage>
)

export default LoginPage
