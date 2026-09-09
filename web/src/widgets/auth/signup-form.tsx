'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { toast } from 'sonner'
import { Button } from '@shared/ui/button'
import { Field, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { useRegister } from '@entities/auth.query'

const signupSchema = z
    .object({
        id: z
            .string()
            .min(3, '아이디는 3자 이상이어야 합니다')
            .max(64)
            .regex(/^[a-zA-Z0-9_-]+$/, '영문·숫자·-·_ 만 사용할 수 있습니다'),
        email: z.email('이메일 형식이 올바르지 않습니다'),
        name: z.string().min(1, '표시 이름을 입력하세요').max(64),
        password: z.string().min(8, '비밀번호는 8자 이상이어야 합니다'),
        passwordConfirm: z.string(),
    })
    .refine((values) => values.password === values.passwordConfirm, { path: ['passwordConfirm'], message: '비밀번호가 일치하지 않습니다' })

type SignupValues = z.infer<typeof signupSchema>

export const SignupForm: FC = () => {
    const router = useRouter()
    const register = useRegister()
    const form = useForm<SignupValues>({
        resolver: zodResolver(signupSchema),
        defaultValues: { id: '', email: '', name: '', password: '', passwordConfirm: '' },
    })

    const submit = form.handleSubmit((values) =>
        register.mutate(
            { id: values.id, password: values.password, email: values.email, name: values.name },
            {
                onSuccess: (result) => {
                    toast.success(`${result.name} 님으로 가입했습니다.`)
                    router.replace('/settings')
                    router.refresh()
                },
            },
        ),
    )

    const errors = form.formState.errors

    return (
        <form onSubmit={submit} className='flex w-full max-w-sm flex-col gap-4' noValidate>
            <div className='flex flex-col gap-1'>
                <h1 className='text-xl font-extrabold tracking-tight'>가입</h1>
                <p className='text-muted-foreground text-sm'>rbms IR 계정을 만듭니다. 아이디는 IR 플레이어 ID 로 사용됩니다.</p>
            </div>
            <Field data-invalid={errors.id ? true : undefined}>
                <FieldLabel htmlFor='signup-id'>아이디</FieldLabel>
                <Input id='signup-id' autoComplete='username' aria-invalid={errors.id ? true : undefined} {...form.register('id')} />
                {errors.id && <FieldError errors={[errors.id]} />}
            </Field>
            <Field data-invalid={errors.email ? true : undefined}>
                <FieldLabel htmlFor='signup-email'>이메일</FieldLabel>
                <Input
                    id='signup-email'
                    type='email'
                    autoComplete='email'
                    aria-invalid={errors.email ? true : undefined}
                    {...form.register('email')}
                />
                {errors.email && <FieldError errors={[errors.email]} />}
            </Field>
            <Field data-invalid={errors.name ? true : undefined}>
                <FieldLabel htmlFor='signup-name'>표시 이름</FieldLabel>
                <Input id='signup-name' autoComplete='nickname' aria-invalid={errors.name ? true : undefined} {...form.register('name')} />
                {errors.name && <FieldError errors={[errors.name]} />}
            </Field>
            <Field data-invalid={errors.password ? true : undefined}>
                <FieldLabel htmlFor='signup-password'>비밀번호</FieldLabel>
                <Input
                    id='signup-password'
                    type='password'
                    autoComplete='new-password'
                    aria-invalid={errors.password ? true : undefined}
                    {...form.register('password')}
                />
                {errors.password && <FieldError errors={[errors.password]} />}
            </Field>
            <Field data-invalid={errors.passwordConfirm ? true : undefined}>
                <FieldLabel htmlFor='signup-password-confirm'>비밀번호 확인</FieldLabel>
                <Input
                    id='signup-password-confirm'
                    type='password'
                    autoComplete='new-password'
                    aria-invalid={errors.passwordConfirm ? true : undefined}
                    {...form.register('passwordConfirm')}
                />
                {errors.passwordConfirm && <FieldError errors={[errors.passwordConfirm]} />}
            </Field>
            <Button type='submit' disabled={form.formState.isSubmitting || register.isPending}>
                가입
            </Button>
            <p className='text-muted-foreground text-xs'>
                이미 계정이 있다면{' '}
                <Link href='/login' className='underline'>
                    로그인
                </Link>
                하세요.
            </p>
        </form>
    )
}
