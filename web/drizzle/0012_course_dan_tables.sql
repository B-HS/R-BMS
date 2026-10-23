CREATE TABLE `course_group` (
	`id` varchar(64) NOT NULL,
	`source_id` varchar(64) NOT NULL,
	`name` varchar(255) NOT NULL DEFAULT '',
	`keys` int NOT NULL DEFAULT 0,
	`position` int NOT NULL DEFAULT 0,
	CONSTRAINT `course_group_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `course_group_member` (
	`group_id` varchar(64) NOT NULL,
	`course_hash` varchar(128) NOT NULL,
	`position` int NOT NULL,
	`active` boolean NOT NULL DEFAULT true,
	CONSTRAINT `course_group_member_group_id_course_hash_pk` PRIMARY KEY(`group_id`,`course_hash`)
);
--> statement-breakpoint
CREATE TABLE `course_source` (
	`id` varchar(64) NOT NULL,
	`kind` varchar(16) NOT NULL,
	`name` varchar(255) NOT NULL DEFAULT '',
	`maker` varchar(255) NOT NULL DEFAULT '',
	`page_url` varchar(512) NOT NULL DEFAULT '',
	`file_url` varchar(512) NOT NULL DEFAULT '',
	`retrieved_at` bigint NOT NULL DEFAULT 0,
	`file_sha256` char(64) NOT NULL DEFAULT '',
	`detail` json,
	`updated_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `course_source_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `dan_course` (
	`course_hash` varchar(128) NOT NULL,
	`system` varchar(32) NOT NULL,
	`label` varchar(64) NOT NULL,
	`rank` int NOT NULL,
	CONSTRAINT `dan_course_course_hash` PRIMARY KEY(`course_hash`)
);
--> statement-breakpoint
CREATE TABLE `table_snapshot` (
	`table_id` varchar(64) NOT NULL,
	`header_url` varchar(512) NOT NULL,
	`data_url` varchar(512) NOT NULL,
	`header_sha256` char(64) NOT NULL,
	`data_sha256` char(64) NOT NULL,
	`header_gz` mediumblob NOT NULL,
	`data_gz` mediumblob NOT NULL,
	`entry_count` int NOT NULL,
	`level_count` int NOT NULL,
	`fetched_at` bigint NOT NULL,
	`checked_at` bigint NOT NULL,
	`last_outcome` varchar(64) NOT NULL DEFAULT '',
	CONSTRAINT `table_snapshot_table_id` PRIMARY KEY(`table_id`)
);
--> statement-breakpoint
CREATE TABLE `user_dan` (
	`user_id` varchar(36) NOT NULL,
	`system` varchar(32) NOT NULL,
	`label` varchar(64) NOT NULL,
	`rank` int NOT NULL,
	`course_hash` varchar(128) NOT NULL,
	`course_score_id` varchar(36) NOT NULL,
	`course_gauge` varchar(16) NOT NULL,
	`gauge` tinyint NOT NULL,
	`acquired_at` bigint NOT NULL,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `user_dan_user_id_system_label_pk` PRIMARY KEY(`user_id`,`system`,`label`)
);
--> statement-breakpoint
CREATE TABLE `user_dan_display` (
	`user_id` varchar(36) NOT NULL,
	`system` varchar(32) NOT NULL,
	`label` varchar(64),
	`updated_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `user_dan_display_user_id_system_pk` PRIMARY KEY(`user_id`,`system`)
);
--> statement-breakpoint
ALTER TABLE `course_chart` ADD `md5` char(32) DEFAULT '' NOT NULL;--> statement-breakpoint
ALTER TABLE `course_chart` ADD `title` varchar(255) DEFAULT '' NOT NULL;--> statement-breakpoint
ALTER TABLE `course_score` ADD `gauge` tinyint;--> statement-breakpoint
ALTER TABLE `course_score` ADD `course_gauge` varchar(16);--> statement-breakpoint
ALTER TABLE `course_score` ADD `stages` json;--> statement-breakpoint
ALTER TABLE `user_dan` ADD CONSTRAINT `user_dan_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `user_dan_display` ADD CONSTRAINT `user_dan_display_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `course_group_member_course_idx` ON `course_group_member` (`course_hash`);--> statement-breakpoint
CREATE INDEX `dan_course_system_idx` ON `dan_course` (`system`,`rank`);