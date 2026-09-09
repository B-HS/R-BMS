CREATE TABLE `api_token` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`token_hash` varchar(255) NOT NULL,
	`label` varchar(64) NOT NULL DEFAULT '',
	`last_used_at` bigint,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`revoked` boolean NOT NULL DEFAULT false,
	CONSTRAINT `api_token_id` PRIMARY KEY(`id`),
	CONSTRAINT `api_token_hash_uniq` UNIQUE(`token_hash`)
);
--> statement-breakpoint
CREATE TABLE `chart` (
	`sha256` char(64) NOT NULL,
	`md5` char(32),
	`title` varchar(255) NOT NULL DEFAULT '',
	`subtitle` varchar(255) NOT NULL DEFAULT '',
	`genre` varchar(255) NOT NULL DEFAULT '',
	`artist` varchar(255) NOT NULL DEFAULT '',
	`subartist` varchar(255) NOT NULL DEFAULT '',
	`level` int,
	`total` double,
	`mode` varchar(16) NOT NULL DEFAULT '',
	`lntype` int NOT NULL DEFAULT 0,
	`judge` int NOT NULL DEFAULT 0,
	`minbpm` int NOT NULL DEFAULT 0,
	`maxbpm` int NOT NULL DEFAULT 0,
	`notes` int NOT NULL DEFAULT 0,
	`has_ln` boolean NOT NULL DEFAULT false,
	`has_cn` boolean NOT NULL DEFAULT false,
	`has_hcn` boolean NOT NULL DEFAULT false,
	`has_mine` boolean NOT NULL DEFAULT false,
	`has_random` boolean NOT NULL DEFAULT false,
	`has_stop` boolean NOT NULL DEFAULT false,
	`url` varchar(512),
	`appendurl` varchar(512),
	`extra` json,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`updated_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `chart_sha256` PRIMARY KEY(`sha256`),
	CONSTRAINT `chart_md5_uniq` UNIQUE(`md5`)
);
--> statement-breakpoint
CREATE TABLE `chart_best` (
	`chart_sha256` char(64) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`score_id` varchar(36) NOT NULL,
	`clear` tinyint NOT NULL DEFAULT 0,
	`ex_score` int NOT NULL DEFAULT 0,
	`minbp` int NOT NULL DEFAULT 0,
	`max_combo` int NOT NULL DEFAULT 0,
	`updated_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `chart_best_chart_sha256_user_id_pk` PRIMARY KEY(`chart_sha256`,`user_id`)
);
--> statement-breakpoint
CREATE TABLE `client_build` (
	`sha256` char(64) NOT NULL,
	`version` varchar(32) NOT NULL DEFAULT '',
	`platform` varchar(32) NOT NULL DEFAULT '',
	`channel` varchar(16) NOT NULL DEFAULT 'stable',
	`released_at` bigint NOT NULL DEFAULT 0,
	`trusted` boolean NOT NULL DEFAULT true,
	`note` varchar(255) NOT NULL DEFAULT '',
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `client_build_sha256` PRIMARY KEY(`sha256`)
);
--> statement-breakpoint
CREATE TABLE `course` (
	`course_hash` varchar(128) NOT NULL,
	`name` varchar(255) NOT NULL DEFAULT '',
	`lntype` int NOT NULL DEFAULT 0,
	`constraint` json,
	`trophy` json,
	`extra` json,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `course_course_hash` PRIMARY KEY(`course_hash`)
);
--> statement-breakpoint
CREATE TABLE `course_best` (
	`course_hash` varchar(128) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`course_score_id` varchar(36) NOT NULL,
	`clear` tinyint NOT NULL DEFAULT 0,
	`ex_score` int NOT NULL DEFAULT 0,
	`minbp` int NOT NULL DEFAULT 0,
	`max_combo` int NOT NULL DEFAULT 0,
	`updated_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `course_best_course_hash_user_id_pk` PRIMARY KEY(`course_hash`,`user_id`)
);
--> statement-breakpoint
CREATE TABLE `course_chart` (
	`course_hash` varchar(128) NOT NULL,
	`position` int NOT NULL,
	`chart_sha256` char(64) NOT NULL,
	CONSTRAINT `course_chart_course_hash_position_pk` PRIMARY KEY(`course_hash`,`position`)
);
--> statement-breakpoint
CREATE TABLE `course_score` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36),
	`course_hash` varchar(128) NOT NULL,
	`clear` tinyint NOT NULL DEFAULT 0,
	`epg` int NOT NULL DEFAULT 0,
	`lpg` int NOT NULL DEFAULT 0,
	`egr` int NOT NULL DEFAULT 0,
	`lgr` int NOT NULL DEFAULT 0,
	`egd` int NOT NULL DEFAULT 0,
	`lgd` int NOT NULL DEFAULT 0,
	`ebd` int NOT NULL DEFAULT 0,
	`lbd` int NOT NULL DEFAULT 0,
	`epr` int NOT NULL DEFAULT 0,
	`lpr` int NOT NULL DEFAULT 0,
	`ems` int NOT NULL DEFAULT 0,
	`lms` int NOT NULL DEFAULT 0,
	`ex_score` int NOT NULL DEFAULT 0,
	`max_combo` int NOT NULL DEFAULT 0,
	`gauge_value` float NOT NULL DEFAULT 0,
	`minbp` int NOT NULL DEFAULT 0,
	`trophy` varchar(32),
	`ranked` boolean NOT NULL DEFAULT true,
	`played_at` bigint NOT NULL,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`extra` json,
	CONSTRAINT `course_score_id` PRIMARY KEY(`id`),
	CONSTRAINT `course_score_idempotent_uniq` UNIQUE(`user_id`,`course_hash`,`played_at`)
);
--> statement-breakpoint
CREATE TABLE `difficulty_table` (
	`id` varchar(64) NOT NULL,
	`name` varchar(255) NOT NULL DEFAULT '',
	`url` varchar(512),
	`extra` json,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `difficulty_table_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `replay` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36),
	`chart_sha256` char(64) NOT NULL,
	`score_id` varchar(36),
	`format` varchar(32) NOT NULL DEFAULT '',
	`mode` varchar(16) NOT NULL DEFAULT '',
	`random` varchar(16) NOT NULL DEFAULT 'Off',
	`random_p2` varchar(16),
	`seed` bigint NOT NULL DEFAULT 0,
	`lntype` int NOT NULL DEFAULT 0,
	`offset_ms` int NOT NULL DEFAULT 0,
	`judge_rate` int NOT NULL DEFAULT 100,
	`scratch_auto` boolean NOT NULL DEFAULT false,
	`constant` boolean NOT NULL DEFAULT false,
	`gauge` tinyint NOT NULL DEFAULT 0,
	`client_build_sha256` char(64),
	`event_count` int NOT NULL DEFAULT 0,
	`duration_us` bigint NOT NULL DEFAULT 0,
	`size` int NOT NULL DEFAULT 0,
	`storage_key` varchar(255),
	`data` mediumtext,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `replay_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `rival` (
	`user_id` varchar(36) NOT NULL,
	`rival_id` varchar(36) NOT NULL,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `rival_user_id_rival_id_pk` PRIMARY KEY(`user_id`,`rival_id`)
);
--> statement-breakpoint
CREATE TABLE `score` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36),
	`guest_name` varchar(64),
	`chart_sha256` char(64) NOT NULL,
	`chart_md5` char(32),
	`mode` varchar(16) NOT NULL DEFAULT '',
	`lntype` int NOT NULL DEFAULT 0,
	`clear` tinyint NOT NULL DEFAULT 0,
	`epg` int NOT NULL DEFAULT 0,
	`lpg` int NOT NULL DEFAULT 0,
	`egr` int NOT NULL DEFAULT 0,
	`lgr` int NOT NULL DEFAULT 0,
	`egd` int NOT NULL DEFAULT 0,
	`lgd` int NOT NULL DEFAULT 0,
	`ebd` int NOT NULL DEFAULT 0,
	`lbd` int NOT NULL DEFAULT 0,
	`epr` int NOT NULL DEFAULT 0,
	`lpr` int NOT NULL DEFAULT 0,
	`ems` int NOT NULL DEFAULT 0,
	`lms` int NOT NULL DEFAULT 0,
	`pgreat` int NOT NULL DEFAULT 0,
	`great` int NOT NULL DEFAULT 0,
	`good` int NOT NULL DEFAULT 0,
	`bad` int NOT NULL DEFAULT 0,
	`poor` int NOT NULL DEFAULT 0,
	`miss` int NOT NULL DEFAULT 0,
	`fast` int NOT NULL DEFAULT 0,
	`slow` int NOT NULL DEFAULT 0,
	`combobreak` int NOT NULL DEFAULT 0,
	`empty_poor` int NOT NULL DEFAULT 0,
	`avgjudge` bigint NOT NULL DEFAULT 0,
	`ex_score` int NOT NULL DEFAULT 0,
	`max_ex_score` int NOT NULL DEFAULT 0,
	`max_combo` int NOT NULL DEFAULT 0,
	`notes` int NOT NULL DEFAULT 0,
	`passnotes` int NOT NULL DEFAULT 0,
	`minbp` int NOT NULL DEFAULT 0,
	`gauge_value` float NOT NULL DEFAULT 0,
	`gauge` tinyint NOT NULL DEFAULT 0,
	`option` bigint NOT NULL DEFAULT 0,
	`random` varchar(16) NOT NULL DEFAULT 'Off',
	`random_p2` varchar(16),
	`scratch_left` boolean NOT NULL DEFAULT false,
	`scratch_auto` boolean NOT NULL DEFAULT false,
	`seed` bigint NOT NULL DEFAULT 0,
	`hispeed` double NOT NULL DEFAULT 0,
	`constant` boolean NOT NULL DEFAULT false,
	`green_number` int,
	`lift` float NOT NULL DEFAULT 0,
	`lane_cover` float NOT NULL DEFAULT 0,
	`assist` int NOT NULL DEFAULT 0,
	`judge_rate` int NOT NULL DEFAULT 100,
	`offset_ms` int NOT NULL DEFAULT 0,
	`auto_offset` boolean NOT NULL DEFAULT false,
	`total_override` double NOT NULL DEFAULT 0,
	`autoplay` boolean NOT NULL DEFAULT false,
	`input_device` varchar(32) NOT NULL DEFAULT '',
	`judge_algorithm` varchar(16) NOT NULL DEFAULT '',
	`rule` varchar(16) NOT NULL DEFAULT '',
	`skin` varchar(64) NOT NULL DEFAULT '',
	`client` varchar(64) NOT NULL DEFAULT '',
	`client_build_sha256` char(64),
	`client_platform` varchar(32),
	`ranked` boolean NOT NULL DEFAULT true,
	`flags` json,
	`verified` boolean NOT NULL DEFAULT false,
	`replay_id` varchar(36),
	`played_at` bigint NOT NULL,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`extra` json,
	CONSTRAINT `score_id` PRIMARY KEY(`id`),
	CONSTRAINT `score_idempotent_uniq` UNIQUE(`user_id`,`chart_sha256`,`played_at`)
);
--> statement-breakpoint
CREATE TABLE `setting_blob` (
	`user_id` varchar(36) NOT NULL,
	`name` varchar(64) NOT NULL,
	`format` varchar(16) NOT NULL DEFAULT '',
	`content` mediumtext NOT NULL,
	`size` int NOT NULL DEFAULT 0,
	`updated_at` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `setting_blob_user_id_name_pk` PRIMARY KEY(`user_id`,`name`)
);
--> statement-breakpoint
CREATE TABLE `submission_audit` (
	`id` varchar(36) NOT NULL,
	`score_id` varchar(36),
	`user_id` varchar(36),
	`client_build_sha256` char(64),
	`client_platform` varchar(32),
	`ip` varchar(64),
	`user_agent` varchar(255),
	`accepted` boolean NOT NULL DEFAULT false,
	`ranked` boolean NOT NULL DEFAULT false,
	`flags` json,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `submission_audit_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `table_chart` (
	`folder_id` varchar(64) NOT NULL,
	`chart_sha256` char(64) NOT NULL,
	CONSTRAINT `table_chart_folder_id_chart_sha256_pk` PRIMARY KEY(`folder_id`,`chart_sha256`)
);
--> statement-breakpoint
CREATE TABLE `table_course` (
	`table_id` varchar(64) NOT NULL,
	`course_hash` varchar(128) NOT NULL,
	CONSTRAINT `table_course_table_id_course_hash_pk` PRIMARY KEY(`table_id`,`course_hash`)
);
--> statement-breakpoint
CREATE TABLE `table_folder` (
	`id` varchar(64) NOT NULL,
	`table_id` varchar(64) NOT NULL,
	`name` varchar(255) NOT NULL DEFAULT '',
	`position` int NOT NULL DEFAULT 0,
	CONSTRAINT `table_folder_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `account` (
	`id` varchar(36) NOT NULL,
	`account_id` varchar(255) NOT NULL,
	`provider_id` varchar(255) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`access_token` text,
	`refresh_token` text,
	`id_token` text,
	`access_token_expires_at` timestamp(3),
	`refresh_token_expires_at` timestamp(3),
	`scope` text,
	`password` text,
	`created_at` timestamp(3) NOT NULL,
	`updated_at` timestamp(3) NOT NULL,
	CONSTRAINT `account_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `session` (
	`id` varchar(36) NOT NULL,
	`expires_at` timestamp(3) NOT NULL,
	`token` varchar(255) NOT NULL,
	`created_at` timestamp(3) NOT NULL,
	`updated_at` timestamp(3) NOT NULL,
	`ip_address` text,
	`user_agent` text,
	`user_id` varchar(36) NOT NULL,
	CONSTRAINT `session_id` PRIMARY KEY(`id`),
	CONSTRAINT `session_token_unique` UNIQUE(`token`)
);
--> statement-breakpoint
CREATE TABLE `user` (
	`id` varchar(36) NOT NULL,
	`name` text NOT NULL,
	`email` varchar(255) NOT NULL,
	`email_verified` boolean NOT NULL,
	`image` text,
	`created_at` timestamp(3) NOT NULL,
	`updated_at` timestamp(3) NOT NULL,
	`login_id` varchar(64) NOT NULL,
	`rank` varchar(32) NOT NULL DEFAULT '',
	`rank_points` double NOT NULL DEFAULT 0,
	`total_plays` bigint NOT NULL DEFAULT 0,
	`role` varchar(16) NOT NULL DEFAULT 'user',
	`allow_guest_merge` boolean NOT NULL DEFAULT false,
	`extra` json,
	CONSTRAINT `user_id` PRIMARY KEY(`id`),
	CONSTRAINT `user_email_unique` UNIQUE(`email`),
	CONSTRAINT `user_login_id_unique` UNIQUE(`login_id`)
);
--> statement-breakpoint
CREATE TABLE `verification` (
	`id` varchar(36) NOT NULL,
	`identifier` varchar(255) NOT NULL,
	`value` text NOT NULL,
	`expires_at` timestamp(3) NOT NULL,
	`created_at` timestamp(3),
	`updated_at` timestamp(3),
	CONSTRAINT `verification_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
ALTER TABLE `api_token` ADD CONSTRAINT `api_token_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `score` ADD CONSTRAINT `score_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `account` ADD CONSTRAINT `account_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `session` ADD CONSTRAINT `session_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `api_token_user_idx` ON `api_token` (`user_id`);--> statement-breakpoint
CREATE INDEX `chart_title_idx` ON `chart` (`title`);--> statement-breakpoint
CREATE INDEX `chart_best_rank_idx` ON `chart_best` (`chart_sha256`,`clear`,`ex_score`);--> statement-breakpoint
CREATE INDEX `course_score_rank_idx` ON `course_score` (`course_hash`,`ranked`,`clear`,`ex_score`);--> statement-breakpoint
CREATE INDEX `replay_chart_idx` ON `replay` (`chart_sha256`);--> statement-breakpoint
CREATE INDEX `replay_score_idx` ON `replay` (`score_id`);--> statement-breakpoint
CREATE INDEX `score_ranking_idx` ON `score` (`chart_sha256`,`ranked`,`clear`,`ex_score`);--> statement-breakpoint
CREATE INDEX `score_player_idx` ON `score` (`user_id`,`played_at`);--> statement-breakpoint
CREATE INDEX `score_build_idx` ON `score` (`client_build_sha256`);--> statement-breakpoint
CREATE INDEX `score_recent_idx` ON `score` (`played_at`);--> statement-breakpoint
CREATE INDEX `submission_audit_build_idx` ON `submission_audit` (`client_build_sha256`);--> statement-breakpoint
CREATE INDEX `submission_audit_created_idx` ON `submission_audit` (`created_at`);--> statement-breakpoint
CREATE INDEX `table_folder_table_idx` ON `table_folder` (`table_id`);