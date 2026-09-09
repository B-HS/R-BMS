'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useRouter, useSearchParams } from 'next/navigation'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { toast } from 'sonner'
import { Button } from '@shared/ui/button'
import { Field, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { useLogin } from '@entities/auth.query'

const loginSchema = z.object({
    id: z.string().min(1, '아이디를 입력하세요').max(64),
    password: z.string().min(8, '비밀번호는 8자 이상이어야 합니다'),
})

type LoginValues = z.infer<typeof loginSchema>

const DEFAULT_NEXT_PATH = '/'

const isSameOriginPath = (path: string) => path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')

export const LoginForm: FC = () => {
    const router = useRouter()
    const searchParams = useSearchParams()
    const login = useLogin()
    const form = useForm<LoginValues>({ resolver: zodResolver(loginSchema), defaultValues: { id: '', password: '' } })

    const nextPath = searchParams.get('next') ?? DEFAULT_NEXT_PATH
    const redirectTarget = isSameOriginPath(nextPath) ? nextPath : DEFAULT_NEXT_PATH

    const submit = form.handleSubmit((values) =>
        login.mutate(values, {
            onSuccess: (result) => {
                toast.success(`${result.name} 님으로 로그인했습니다.`)
                router.replace(redirectTarget)
                router.refresh()
            },
        }),
    )

    return (
        <form onSubmit={submit} className='flex w-full max-w-sm flex-col gap-4' noValidate>
            <div className='flex flex-col gap-1'>
                <h1 className='text-xl font-extrabold tracking-tight'>로그인</h1>
                <p className='text-muted-foreground text-sm'>rbms IR 계정으로 로그인합니다.</p>
            </div>
            <Field data-invalid={form.formState.errors.id ? true : undefined}>
                <FieldLabel htmlFor='login-id'>아이디</FieldLabel>
                <Input id='login-id' autoComplete='username' aria-invalid={form.formState.errors.id ? true : undefined} {...form.register('id')} />
                {form.formState.errors.id && <FieldError errors={[form.formState.errors.id]} />}
            </Field>
            <Field data-invalid={form.formState.errors.password ? true : undefined}>
                <FieldLabel htmlFor='login-password'>비밀번호</FieldLabel>
                <Input
                    id='login-password'
                    type='password'
                    autoComplete='current-password'
                    aria-invalid={form.formState.errors.password ? true : undefined}
                    {...form.register('password')}
                />
                {form.formState.errors.password && <FieldError errors={[form.formState.errors.password]} />}
            </Field>
            <Button type='submit' disabled={form.formState.isSubmitting || login.isPending}>
                로그인
            </Button>
            <p className='text-muted-foreground text-xs'>
                계정이 없다면{' '}
                <Link href='/signup' className='underline'>
                    가입
                </Link>
                하거나{' '}
                <Link href='/guide' className='underline'>
                    연동 가이드
                </Link>
                를 확인하세요.
            </p>
        </form>
    )
}
