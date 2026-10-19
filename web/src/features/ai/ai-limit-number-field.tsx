import type { FC } from 'react'
import { Field, FieldDescription, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'

type AiLimitNumberFieldProps = {
    id: string
    label: string
    value: string
    onValueChange: (value: string) => void
    description: string
    error?: string
    placeholder?: string
    disabled?: boolean
}

export const AiLimitNumberField: FC<AiLimitNumberFieldProps> = ({ id, label, value, onValueChange, description, error, placeholder, disabled }) => (
    <Field data-invalid={Boolean(error)}>
        <FieldLabel htmlFor={id}>{label}</FieldLabel>
        <Input
            id={id}
            inputMode='numeric'
            autoComplete='off'
            value={value}
            placeholder={placeholder}
            disabled={disabled}
            aria-invalid={Boolean(error)}
            aria-describedby={`${id}-description`}
            onChange={(event) => onValueChange(event.target.value)}
        />
        <FieldDescription id={`${id}-description`}>{description}</FieldDescription>
        {error && <FieldError>{error}</FieldError>}
    </Field>
)
