CREATE TABLE `play_profile` (
	`user_id` varchar(36) NOT NULL,
	`window_key` varchar(16) NOT NULL,
	`profile_version` int NOT NULL,
	`fingerprint` char(64) NOT NULL,
	`data` json NOT NULL,
	`computed_at` bigint NOT NULL,
	CONSTRAINT `play_profile_user_id_window_key_pk` PRIMARY KEY(`user_id`,`window_key`)
);
--> statement-breakpoint
CREATE TABLE `training_chart` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`md5` char(32) NOT NULL,
	`sha256` char(64) NOT NULL,
	`title` varchar(200) NOT NULL,
	`play_level` int NOT NULL,
	`bpm` int NOT NULL,
	`note_count` int NOT NULL,
	`length_ms` int NOT NULL,
	`generator_version` int NOT NULL,
	`spec` json NOT NULL,
	`request` json NOT NULL,
	`profile_snapshot` json NOT NULL,
	`bms` mediumblob NOT NULL,
	`provider` varchar(16) NOT NULL,
	`model` varchar(128) NOT NULL,
	`created_at` bigint NOT NULL,
	CONSTRAINT `training_chart_id` PRIMARY KEY(`id`),
	CONSTRAINT `training_chart_md5_uniq` UNIQUE(`md5`)
);
--> statement-breakpoint
CREATE TABLE `training_folder` (
	`user_id` varchar(36) NOT NULL,
	`folder_id` varchar(36) NOT NULL,
	`created_at` bigint NOT NULL,
	CONSTRAINT `training_folder_user_id` PRIMARY KEY(`user_id`)
);
--> statement-breakpoint
ALTER TABLE `play_profile` ADD CONSTRAINT `play_profile_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `training_chart` ADD CONSTRAINT `training_chart_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `training_folder` ADD CONSTRAINT `training_folder_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `training_folder` ADD CONSTRAINT `training_folder_folder_id_user_folder_id_fk` FOREIGN KEY (`folder_id`) REFERENCES `user_folder`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `training_chart_sha256_idx` ON `training_chart` (`sha256`);--> statement-breakpoint
CREATE INDEX `training_chart_user_created_idx` ON `training_chart` (`user_id`,`created_at`);