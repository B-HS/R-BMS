'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { toast } from 'sonner'
import { Button } from '@shared/ui/button'
import { Field, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { DEFAULT_NEXT_PATH, isSameOriginPath, NEXT_PARAM } from '@shared/constants/auth-route'
import { useSessionLogin } from '@entities/auth.query'

const MIN_PASSWORD_LENGTH = 8
const MAX_LOGIN_ID_LENGTH = 64

const loginSchema = z.object({
    id: z.string().min(1, '아이디를 입력하세요').max(MAX_LOGIN_ID_LENGTH),
    password: z.string().min(MIN_PASSWORD_LENGTH, `비밀번호는 ${MIN_PASSWORD_LENGTH}자 이상이어야 합니다`),
})

type LoginValues = z.infer<typeof loginSchema>

const readRedirectTarget = () => {
    const nextPath = new URLSearchParams(window.location.search).get(NEXT_PARAM) ?? DEFAULT_NEXT_PATH
    return isSameOriginPath(nextPath) ? nextPath : DEFAULT_NEXT_PATH
}

export const LoginForm: FC = () => {
    const login = useSessionLogin()
    const form = useForm<LoginValues>({ resolver: zodResolver(loginSchema), defaultValues: { id: '', password: '' } })

    const errors = form.formState.errors

    const handleSubmit = form.handleSubmit((values) =>
        login.mutate(values, {
            onSuccess: (user) => {
                toast.success(`${user.name} 님으로 로그인했습니다.`)
                window.location.assign(readRedirectTarget())
            },
        }),
    )

    return (
        <form onSubmit={handleSubmit} className='flex flex-col gap-4' noValidate>
            <Field data-invalid={errors.id ? true : undefined}>
                <FieldLabel htmlFor='login-id'>아이디</FieldLabel>
                <Input id='login-id' autoComplete='username' aria-invalid={errors.id ? true : undefined} {...form.register('id')} />
                {errors.id && <FieldError errors={[errors.id]} />}
            </Field>
            <Field data-invalid={errors.password ? true : undefined}>
                <FieldLabel htmlFor='login-password'>비밀번호</FieldLabel>
                <Input
                    id='login-password'
                    type='password'
                    autoComplete='current-password'
                    aria-invalid={errors.password ? true : undefined}
                    {...form.register('password')}
                />
                {errors.password && <FieldError errors={[errors.password]} />}
            </Field>
            {login.isError && (
                <p role='alert' className='text-destructive text-xs'>
                    {login.error.message}
                </p>
            )}
            <div className='flex flex-col gap-1'>
                <Button type='submit' className='w-full' disabled={form.formState.isSubmitting || login.isPending}>
                    로그인
                </Button>
                <Button asChild variant='ghost' className='w-full'>
                    <Link href='/signup'>계정이 없으신가요? 가입</Link>
                </Button>
            </div>
            <p className='text-muted-foreground text-xs'>
                게임 안에서 계정을 연결하는 방법은{' '}
                <Link href='/guide' className='underline'>
                    가이드
                </Link>
                에 있습니다.
            </p>
        </form>
    )
}
