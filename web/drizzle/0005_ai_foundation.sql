CREATE TABLE `ai_credential` (
	`user_id` varchar(36) NOT NULL,
	`provider` varchar(16) NOT NULL,
	`ciphertext` varchar(1024) NOT NULL,
	`iv` varchar(24) NOT NULL,
	`auth_tag` varchar(32) NOT NULL,
	`wrapped_dek` varchar(64) NOT NULL,
	`dek_iv` varchar(24) NOT NULL,
	`dek_tag` varchar(32) NOT NULL,
	`kek_version` varchar(32) NOT NULL,
	`key_hint` varchar(8) NOT NULL,
	`model` varchar(128),
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`updated_at` bigint NOT NULL DEFAULT 0,
	`last_validated_at` bigint,
	`last_error_kind` varchar(32),
	CONSTRAINT `ai_credential_user_id_provider_pk` PRIMARY KEY(`user_id`,`provider`)
);
--> statement-breakpoint
CREATE TABLE `ai_inflight` (
	`user_id` varchar(36) NOT NULL,
	`slot` int NOT NULL,
	`run_id` varchar(36) NOT NULL,
	`expires_at` bigint NOT NULL,
	CONSTRAINT `ai_inflight_user_id_slot_pk` PRIMARY KEY(`user_id`,`slot`)
);
--> statement-breakpoint
CREATE TABLE `ai_result` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`feature` varchar(32) NOT NULL,
	`subject_key` varchar(128) NOT NULL,
	`input_hash` char(64) NOT NULL,
	`analysis_version` int NOT NULL DEFAULT 0,
	`prompt_version` int NOT NULL DEFAULT 0,
	`provider` varchar(16) NOT NULL,
	`model` varchar(128) NOT NULL,
	`output` json NOT NULL,
	`usage` json NOT NULL,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `ai_result_id` PRIMARY KEY(`id`),
	CONSTRAINT `ai_result_input_uniq` UNIQUE(`user_id`,`feature`,`subject_key`,`input_hash`)
);
--> statement-breakpoint
CREATE TABLE `ai_setting` (
	`id` varchar(16) NOT NULL,
	`daily_calls` int NOT NULL,
	`concurrent_runs` int NOT NULL,
	`default_monthly_token_limit` bigint,
	`updated_at` bigint NOT NULL DEFAULT 0,
	`updated_by` varchar(36),
	CONSTRAINT `ai_setting_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `ai_usage` (
	`user_id` varchar(36) NOT NULL,
	`day` char(10) NOT NULL,
	`calls` int NOT NULL DEFAULT 0,
	`input_tokens` bigint NOT NULL DEFAULT 0,
	`output_tokens` bigint NOT NULL DEFAULT 0,
	`cached_input_tokens` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `ai_usage_user_id_day_pk` PRIMARY KEY(`user_id`,`day`)
);
--> statement-breakpoint
CREATE TABLE `ai_user_setting` (
	`user_id` varchar(36) NOT NULL,
	`monthly_token_limit` bigint,
	`default_provider` varchar(16),
	`updated_at` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `ai_user_setting_user_id` PRIMARY KEY(`user_id`)
);
--> statement-breakpoint
ALTER TABLE `ai_credential` ADD CONSTRAINT `ai_credential_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_inflight` ADD CONSTRAINT `ai_inflight_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_result` ADD CONSTRAINT `ai_result_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_usage` ADD CONSTRAINT `ai_usage_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_user_setting` ADD CONSTRAINT `ai_user_setting_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `ai_result_recent_idx` ON `ai_result` (`user_id`,`feature`,`created_at`);--> statement-breakpoint
CREATE INDEX `chart_best_user_idx` ON `chart_best` (`user_id`,`scratch_auto`);