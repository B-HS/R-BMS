import { BUILD_TRUST, type BuildTrust } from '@server/service/domain/score/ranked-policy'

export type BuildAllowlistDb = {
    findBuild: (sha256: string) => Promise<{ trusted: boolean } | null>
}

export const resolveBuildTrust = async (deps: { db: BuildAllowlistDb }, sha256: string | null): Promise<BuildTrust> => {
    if (!sha256) return BUILD_TRUST.UNKNOWN
    const build = await deps.db.findBuild(sha256)
    if (!build) return BUILD_TRUST.UNKNOWN
    return build.trusted ? BUILD_TRUST.TRUSTED : BUILD_TRUST.UNTRUSTED
}
