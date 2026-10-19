CREATE TABLE `ai_goal` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`kind` varchar(32) NOT NULL,
	`params` json NOT NULL,
	`created_at` bigint NOT NULL,
	`archived_at` bigint,
	CONSTRAINT `ai_goal_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `ai_practice_folder` (
	`user_id` varchar(36) NOT NULL,
	`preview_hash` char(64) NOT NULL,
	`folder_id` varchar(36) NOT NULL,
	`created_at` bigint NOT NULL,
	CONSTRAINT `ai_practice_folder_user_id_preview_hash_pk` PRIMARY KEY(`user_id`,`preview_hash`)
);
--> statement-breakpoint
ALTER TABLE `ai_goal` ADD CONSTRAINT `ai_goal_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_practice_folder` ADD CONSTRAINT `ai_practice_folder_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `ai_practice_folder` ADD CONSTRAINT `ai_practice_folder_folder_id_user_folder_id_fk` FOREIGN KEY (`folder_id`) REFERENCES `user_folder`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `ai_goal_user_idx` ON `ai_goal` (`user_id`,`archived_at`);