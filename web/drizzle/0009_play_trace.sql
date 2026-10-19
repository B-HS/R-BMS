CREATE TABLE `play_analysis` (
	`trace_id` varchar(36) NOT NULL,
	`analysis_version` int NOT NULL,
	`summary` json NOT NULL,
	`pattern_join` varchar(16) NOT NULL,
	`pattern_md5` char(32),
	`computed_at` bigint NOT NULL,
	CONSTRAINT `play_analysis_trace_id` PRIMARY KEY(`trace_id`)
);
--> statement-breakpoint
CREATE TABLE `play_trace` (
	`id` varchar(36) NOT NULL,
	`score_id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`chart_sha256` char(64) NOT NULL,
	`chart_md5` char(32),
	`played_at` bigint NOT NULL,
	`format_version` int NOT NULL,
	`note_count` int NOT NULL DEFAULT 0,
	`byte_size` int NOT NULL DEFAULT 0,
	`stored_size` int NOT NULL DEFAULT 0,
	`data` mediumblob NOT NULL,
	`created_at` bigint NOT NULL,
	CONSTRAINT `play_trace_id` PRIMARY KEY(`id`),
	CONSTRAINT `play_trace_score_uniq` UNIQUE(`score_id`)
);
--> statement-breakpoint
ALTER TABLE `play_analysis` ADD CONSTRAINT `play_analysis_trace_id_play_trace_id_fk` FOREIGN KEY (`trace_id`) REFERENCES `play_trace`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `play_trace` ADD CONSTRAINT `play_trace_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `play_analysis_version_idx` ON `play_analysis` (`analysis_version`);--> statement-breakpoint
CREATE INDEX `play_trace_user_played_idx` ON `play_trace` (`user_id`,`played_at`);--> statement-breakpoint
CREATE INDEX `play_trace_chart_user_idx` ON `play_trace` (`chart_sha256`,`user_id`);