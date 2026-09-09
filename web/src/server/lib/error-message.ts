import { ERROR_CODE, type ErrorCode } from '@server/lib/error-code'

export const ERROR_MESSAGE: Record<ErrorCode, string> = {
    [ERROR_CODE.VALIDATION_ERROR]: '요청 형식이 올바르지 않습니다.',
    [ERROR_CODE.UNAUTHORIZED]: '인증이 필요합니다.',
    [ERROR_CODE.FORBIDDEN]: '권한이 없습니다.',
    [ERROR_CODE.NOT_FOUND]: '요청한 리소스를 찾을 수 없습니다.',
    [ERROR_CODE.RATE_LIMITED]: '요청이 너무 많습니다. 잠시 후 다시 시도해 주세요.',
    [ERROR_CODE.SERVICE_NOT_CONFIGURED]: '해당 기능이 서버에 구성되어 있지 않습니다.',
    [ERROR_CODE.INTERNAL_ERROR]: '서버 내부 오류가 발생했습니다.',
    [ERROR_CODE.IR_CHART_NOT_FOUND]: '차트를 찾을 수 없습니다.',
    [ERROR_CODE.IR_PLAYER_NOT_FOUND]: '플레이어를 찾을 수 없습니다.',
    [ERROR_CODE.IR_SCORE_NOT_FOUND]: '스코어를 찾을 수 없습니다.',
    [ERROR_CODE.IR_REPLAY_NOT_FOUND]: '리플레이를 찾을 수 없습니다.',
    [ERROR_CODE.IR_SETTING_NOT_FOUND]: '설정 blob 을 찾을 수 없습니다.',
    [ERROR_CODE.IR_COURSE_NOT_FOUND]: '코스를 찾을 수 없습니다.',
    [ERROR_CODE.IR_TABLE_NOT_FOUND]: '난이도표를 찾을 수 없습니다.',
    [ERROR_CODE.IR_ACCOUNT_EXISTS]: '이미 존재하는 계정입니다.',
    [ERROR_CODE.IR_PAYLOAD_TOO_LARGE]: '요청 본문이 허용 크기를 초과했습니다.',
    [ERROR_CODE.IR_API_VERSION_UNSUPPORTED]: '지원하지 않는 api_version 입니다.',
}
