'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
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

type LoginTranslator = ReturnType<typeof useTranslations<'browse.auth'>>

const buildLoginSchema = (t: LoginTranslator) =>
    z.object({
        id: z.string().min(1, t('errors.idRequired')).max(MAX_LOGIN_ID_LENGTH),
        password: z.string().min(MIN_PASSWORD_LENGTH, t('errors.passwordMin', { min: MIN_PASSWORD_LENGTH })),
    })

type LoginValues = z.infer<ReturnType<typeof buildLoginSchema>>

const readRedirectTarget = () => {
    const nextPath = new URLSearchParams(window.location.search).get(NEXT_PARAM) ?? DEFAULT_NEXT_PATH
    return isSameOriginPath(nextPath) ? nextPath : DEFAULT_NEXT_PATH
}

export const LoginForm: FC = () => {
    const t = useTranslations('browse.auth')
    const login = useSessionLogin()
    const form = useForm<LoginValues>({ resolver: zodResolver(buildLoginSchema(t)), defaultValues: { id: '', password: '' } })

    const errors = form.formState.errors

    const handleSubmit = form.handleSubmit((values) =>
        login.mutate(values, {
            onSuccess: (user) => {
                toast.success(t('loggedIn', { name: user.name }))
                window.location.assign(readRedirectTarget())
            },
        }),
    )

    return (
        <form onSubmit={handleSubmit} className='flex flex-col gap-4' noValidate>
            <Field data-invalid={errors.id ? true : undefined}>
                <FieldLabel htmlFor='login-id'>{t('fields.id')}</FieldLabel>
                <Input id='login-id' autoComplete='username' aria-invalid={errors.id ? true : undefined} {...form.register('id')} />
                {errors.id && <FieldError errors={[errors.id]} />}
            </Field>
            <Field data-invalid={errors.password ? true : undefined}>
                <FieldLabel htmlFor='login-password'>{t('fields.password')}</FieldLabel>
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
                    {t('login')}
                </Button>
                <Button asChild variant='ghost' className='w-full'>
                    <Link href='/signup'>{t('toSignup')}</Link>
                </Button>
            </div>
            <p className='text-muted-foreground text-xs'>
                {t.rich('guideHint', {
                    guide: (chunks) => (
                        <Link href='/guide' className='underline'>
                            {chunks}
                        </Link>
                    ),
                })}
            </p>
        </form>
    )
}
