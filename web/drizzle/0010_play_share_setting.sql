CREATE TABLE `play_share_setting` (
	`user_id` varchar(36) NOT NULL,
	`rival_segments` boolean NOT NULL DEFAULT false,
	`updated_at` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `play_share_setting_user_id` PRIMARY KEY(`user_id`)
);
--> statement-breakpoint
ALTER TABLE `play_share_setting` ADD CONSTRAINT `play_share_setting_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;