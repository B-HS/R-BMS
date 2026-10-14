import { buildPageMetadata } from '@shared/lib/metadata'
import { AuthCard } from '@widgets/auth/auth-card'
import { SignupForm } from '@widgets/auth/signup-form'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '가입', description: 'R-BMS 계정을 만듭니다.', path: '/signup' })

const SignupPage = () => (
    <ShellPage title='가입' contentClassName='gap-0'>
        <AuthCard title='가입' description='R-BMS 계정을 만듭니다. 아이디는 순위에 표시되는 플레이어 ID 로 쓰입니다.'>
            <SignupForm />
        </AuthCard>
    </ShellPage>
)

export default SignupPage
