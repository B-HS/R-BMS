CREATE TABLE `user_folder` (
	`id` varchar(36) NOT NULL,
	`user_id` varchar(36) NOT NULL,
	`name` varchar(64) NOT NULL,
	`description` varchar(255) NOT NULL DEFAULT '',
	`is_public` boolean NOT NULL DEFAULT false,
	`position` int NOT NULL DEFAULT 0,
	`item_count` int NOT NULL DEFAULT 0,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`updated_at` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `user_folder_id` PRIMARY KEY(`id`),
	CONSTRAINT `user_folder_owner_name_uniq` UNIQUE(`user_id`,`name`)
);
--> statement-breakpoint
CREATE TABLE `user_folder_item` (
	`folder_id` varchar(36) NOT NULL,
	`md5` char(32) NOT NULL,
	`sha256` char(64),
	`title` varchar(200) NOT NULL DEFAULT '',
	`artist` varchar(200) NOT NULL DEFAULT '',
	`note` varchar(100) NOT NULL DEFAULT '',
	`position` int NOT NULL DEFAULT 0,
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `user_folder_item_folder_id_md5_pk` PRIMARY KEY(`folder_id`,`md5`)
);
--> statement-breakpoint
ALTER TABLE `user_folder` ADD CONSTRAINT `user_folder_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `user_folder_item` ADD CONSTRAINT `user_folder_item_folder_id_user_folder_id_fk` FOREIGN KEY (`folder_id`) REFERENCES `user_folder`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX `user_folder_owner_position_idx` ON `user_folder` (`user_id`,`position`);--> statement-breakpoint
CREATE INDEX `user_folder_item_position_idx` ON `user_folder_item` (`folder_id`,`position`);--> statement-breakpoint
CREATE INDEX `user_folder_item_md5_idx` ON `user_folder_item` (`md5`);