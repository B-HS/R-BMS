'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { toast } from 'sonner'
import { Button } from '@shared/ui/button'
import { Field, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { useRegister } from '@entities/auth.query'

const MIN_ID_LENGTH = 3
const MIN_PASSWORD_LENGTH = 8

type SignupTranslator = ReturnType<typeof useTranslations<'browse.auth'>>

const buildSignupSchema = (t: SignupTranslator) =>
    z
        .object({
            id: z
                .string()
                .min(MIN_ID_LENGTH, t('errors.idMin', { min: MIN_ID_LENGTH }))
                .max(64)
                .regex(/^[a-zA-Z0-9_-]+$/, t('errors.idPattern')),
            email: z.email(t('errors.emailInvalid')),
            name: z.string().min(1, t('errors.nameRequired')).max(64),
            password: z.string().min(MIN_PASSWORD_LENGTH, t('errors.passwordMin', { min: MIN_PASSWORD_LENGTH })),
            passwordConfirm: z.string(),
        })
        .refine((values) => values.password === values.passwordConfirm, { path: ['passwordConfirm'], message: t('errors.passwordMismatch') })

type SignupValues = z.infer<ReturnType<typeof buildSignupSchema>>

export const SignupForm: FC = () => {
    const router = useRouter()
    const t = useTranslations('browse.auth')
    const register = useRegister()
    const form = useForm<SignupValues>({
        resolver: zodResolver(buildSignupSchema(t)),
        defaultValues: { id: '', email: '', name: '', password: '', passwordConfirm: '' },
    })

    const submit = form.handleSubmit((values) =>
        register.mutate(
            { id: values.id, password: values.password, email: values.email, name: values.name },
            {
                onSuccess: (user) => {
                    toast.success(t('signedUp', { name: user.name }))
                    router.replace('/settings')
                    router.refresh()
                },
            },
        ),
    )

    const errors = form.formState.errors

    return (
        <form onSubmit={submit} className='flex flex-col gap-4' noValidate>
            <Field data-invalid={errors.id ? true : undefined}>
                <FieldLabel htmlFor='signup-id'>{t('fields.id')}</FieldLabel>
                <Input id='signup-id' autoComplete='username' aria-invalid={errors.id ? true : undefined} {...form.register('id')} />
                {errors.id && <FieldError errors={[errors.id]} />}
            </Field>
            <Field data-invalid={errors.email ? true : undefined}>
                <FieldLabel htmlFor='signup-email'>{t('fields.email')}</FieldLabel>
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
                <FieldLabel htmlFor='signup-name'>{t('fields.displayName')}</FieldLabel>
                <Input id='signup-name' autoComplete='nickname' aria-invalid={errors.name ? true : undefined} {...form.register('name')} />
                {errors.name && <FieldError errors={[errors.name]} />}
            </Field>
            <Field data-invalid={errors.password ? true : undefined}>
                <FieldLabel htmlFor='signup-password'>{t('fields.password')}</FieldLabel>
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
                <FieldLabel htmlFor='signup-password-confirm'>{t('fields.passwordConfirm')}</FieldLabel>
                <Input
                    id='signup-password-confirm'
                    type='password'
                    autoComplete='new-password'
                    aria-invalid={errors.passwordConfirm ? true : undefined}
                    {...form.register('passwordConfirm')}
                />
                {errors.passwordConfirm && <FieldError errors={[errors.passwordConfirm]} />}
            </Field>
            {register.isError && (
                <p role='alert' className='text-destructive text-xs'>
                    {register.error.message}
                </p>
            )}
            <div className='flex flex-col gap-1'>
                <Button type='submit' className='w-full' disabled={form.formState.isSubmitting || register.isPending}>
                    {t('signup')}
                </Button>
                <Button asChild variant='ghost' className='w-full'>
                    <Link href='/login'>{t('toLogin')}</Link>
                </Button>
            </div>
        </form>
    )
}
