-- The documented WoW Forever 1.60.1.70009 API that Timeways uses.
-- Written by scripts/wow-api.sh from Blizzard_APIDocumentationGenerated. Do not edit.
-- A patch can change the arguments, returns, or secret flags and keep the name. The diff shows it.
-- The scan does not know the type of each object, so methods has each widget type with a called name.
return {
	build = "1.60.1.70009",
	functions = {
		Ambiguate = {
			SecretArguments = "AllowedWhenTainted",
			Arguments = {
				{ Name = "fullName", Type = "cstring", Nilable = false },
				{ Name = "context", Type = "cstring", Nilable = false, NeverSecret = true },
			},
			Returns = {
				{ Name = "result", Type = "string", Nilable = false },
			},
		},
		["C_AddOns.EnableAddOn"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "uiAddon", Nilable = false },
				{ Name = "character", Type = "cstring", Nilable = false, Default = "0" },
			},
		},
		["C_AddOns.IsAddOnLoaded"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "uiAddon", Nilable = false },
			},
			Returns = {
				{ Name = "loadedOrLoading", Type = "bool", Nilable = false },
				{ Name = "loaded", Type = "bool", Nilable = false },
			},
		},
		["C_AddOns.LoadAddOn"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "uiAddon", Nilable = false },
			},
			Returns = {
				{ Name = "loaded", Type = "bool", Nilable = true },
				{ Name = "value", Type = "string", Nilable = true },
			},
		},
		["C_CVar.GetCVar"] = {
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "name", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "value", Type = "string", Nilable = true },
			},
		},
		["C_ChatInfo.RegisterAddonMessagePrefix"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "prefix", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "RegisterAddonMessagePrefixResult", Nilable = false },
			},
		},
		["C_ChatInfo.SendAddonMessage"] = {
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "prefix", Type = "cstring", Nilable = false },
				{ Name = "message", Type = "cstring", Nilable = false },
				{ Name = "chatType", Type = "cstring", Nilable = true },
				{ Name = "target", Type = "cstring", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "SendAddonMessageResult", Nilable = false },
			},
		},
		["C_ChatInfo.SendAddonMessageLogged"] = {
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "prefix", Type = "cstring", Nilable = false },
				{ Name = "message", Type = "cstring", Nilable = false },
				{ Name = "chatType", Type = "cstring", Nilable = true },
				{ Name = "target", Type = "cstring", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "SendAddonMessageResult", Nilable = true },
			},
		},
		["C_Cursor.GetCursorItem"] = {
			MayReturnNothing = true,
			Returns = {
				{ Name = "item", Type = "ItemLocation", Mixin = "ItemLocationMixin", Nilable = false },
			},
		},
		["C_DeathRecap.GetRecapEvents"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "recapID", Type = "number", Nilable = true },
			},
			Returns = {
				{ Name = "events", Type = "table", InnerType = "DeathRecapEventInfo", Nilable = false },
			},
		},
		["C_DeathRecap.HasRecapEvents"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "recapID", Type = "number", Nilable = true },
			},
			Returns = {
				{ Name = "hasEvents", Type = "bool", Nilable = false },
			},
		},
		["C_FriendList.GetFriendInfo"] = {
			MayReturnNothing = true,
			RequiresFriendList = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "info", Type = "FriendInfo", Nilable = false },
			},
		},
		["C_FriendList.GetFriendInfoByIndex"] = {
			MayReturnNothing = true,
			RequiresFriendList = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "index", Type = "luaIndex", Nilable = false },
			},
			Returns = {
				{ Name = "info", Type = "FriendInfo", Nilable = false },
			},
		},
		["C_FriendList.GetNumFriends"] = {
			RequiresFriendList = true,
			Returns = {
				{ Name = "numFriends", Type = "number", Nilable = false },
			},
		},
		["C_GossipInfo.GetText"] = {
			Returns = {
				{ Name = "gossipText", Type = "cstring", Nilable = false },
			},
		},
		["C_GuildInfo.GuildRoster"] = {},
		["C_Item.GetItemCount"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "itemInfo", Type = "ItemInfo", Nilable = false },
				{ Name = "includeBank", Type = "bool", Nilable = false, Default = false },
				{ Name = "includeUses", Type = "bool", Nilable = false, Default = false },
				{ Name = "includeReagentBank", Type = "bool", Nilable = false, Default = false },
				{ Name = "includeAccountBank", Type = "bool", Nilable = false, Default = false },
			},
			Returns = {
				{ Name = "count", Type = "number", Nilable = false },
			},
		},
		["C_Item.GetItemIconByID"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "itemInfo", Type = "ItemInfo", Nilable = false },
			},
			Returns = {
				{ Name = "icon", Type = "fileID", Nilable = true },
			},
		},
		["C_Item.GetItemInfo"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "itemInfo", Type = "ItemInfo", Nilable = false },
			},
			Returns = {
				{ Name = "itemName", Type = "cstring", Nilable = false },
				{ Name = "itemLink", Type = "cstring", Nilable = false },
				{ Name = "itemQuality", Type = "ItemQuality", Nilable = false },
				{ Name = "itemLevel", Type = "number", Nilable = false },
				{ Name = "itemMinLevel", Type = "number", Nilable = false },
				{ Name = "itemType", Type = "cstring", Nilable = false },
				{ Name = "itemSubType", Type = "cstring", Nilable = false },
				{ Name = "itemStackCount", Type = "number", Nilable = false },
				{ Name = "itemEquipLoc", Type = "cstring", Nilable = false },
				{ Name = "itemTexture", Type = "fileID", Nilable = false },
				{ Name = "sellPrice", Type = "number", Nilable = false },
				{ Name = "classID", Type = "number", Nilable = false },
				{ Name = "subclassID", Type = "number", Nilable = false },
				{ Name = "bindType", Type = "number", Nilable = false },
				{ Name = "expansionID", Type = "number", Nilable = false },
				{ Name = "setID", Type = "number", Nilable = true },
				{ Name = "isCraftingReagent", Type = "bool", Nilable = false },
				{ Name = "itemDescription", Type = "cstring", Nilable = false },
			},
		},
		["C_Item.GetStackCount"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "itemLocation", Type = "ItemLocation", Mixin = "ItemLocationMixin", Nilable = false },
			},
			Returns = {
				{ Name = "stackCount", Type = "number", Nilable = false },
			},
		},
		["C_MajorFactions.GetMajorFactionProgressionInfo"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "majorFactionID", Type = "number", Nilable = false },
			},
			Returns = {
				{ Name = "data", Type = "MajorFactionProgressionInfo", Nilable = true },
			},
		},
		["C_Map.GetBestMapForUnit"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unitToken", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "uiMapID", Type = "number", Nilable = true },
			},
		},
		["C_Map.GetFallbackWorldMapID"] = {
			Returns = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
			},
		},
		["C_Map.GetMapArtLayerTextures"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
				{ Name = "layerIndex", Type = "luaIndex", Nilable = false },
			},
			Returns = {
				{ Name = "textures", Type = "table", InnerType = "fileID", Nilable = false },
			},
		},
		["C_Map.GetMapArtLayers"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
			},
			Returns = {
				{ Name = "layerInfo", Type = "table", InnerType = "UiMapLayerInfo", Nilable = false },
			},
		},
		["C_Map.GetMapChildrenInfo"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
				{ Name = "mapType", Type = "UIMapType", Nilable = true },
				{ Name = "allDescendants", Type = "bool", Nilable = true },
			},
			Returns = {
				{ Name = "info", Type = "table", InnerType = "UiMapDetails", Nilable = false },
			},
		},
		["C_Map.GetMapInfo"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
			},
			Returns = {
				{ Name = "info", Type = "UiMapDetails", Nilable = false },
			},
		},
		["C_Map.GetPlayerMapPosition"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
				{ Name = "unitToken", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "position", Type = "vector2", Mixin = "Vector2DMixin", Nilable = true },
			},
		},
		["C_MapExplorationInfo.GetExploredMapTextures"] = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "uiMapID", Type = "number", Nilable = false },
			},
			Returns = {
				{ Name = "overlayInfo", Type = "table", InnerType = "UiMapExplorationInfo", Nilable = false },
			},
		},
		["C_MountJournal.GetMountFromSpell"] = {
			SecretArguments = "AllowedWhenTainted",
			Arguments = {
				{ Name = "spellID", Type = "SpellIdentifier", Nilable = false },
			},
			Returns = {
				{ Name = "mountID", Type = "number", Nilable = true },
			},
		},
		["C_QuestLog.GetInfo"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "questLogIndex", Type = "luaIndex", Nilable = false },
			},
			Returns = {
				{ Name = "info", Type = "QuestInfo", Nilable = true },
			},
		},
		["C_QuestLog.GetNumQuestLogEntries"] = {
			Returns = {
				{ Name = "numShownEntries", Type = "number", Nilable = false },
				{ Name = "numQuests", Type = "number", Nilable = false },
			},
		},
		["C_Timer.After"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "seconds", Type = "number", Nilable = false },
				{ Name = "callback", Type = "TimerCallback", Nilable = false },
			},
		},
		["C_Timer.NewTicker"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "seconds", Type = "number", Nilable = false },
				{ Name = "callback", Type = "TickerCallback", Nilable = false },
				{ Name = "iterations", Type = "number", Nilable = true },
			},
			Returns = {
				{ Name = "cbObject", Type = "TickerCallback", Nilable = false },
			},
		},
		CheckInteractDistance = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unitGUID", Type = "UnitToken", Nilable = false },
				{ Name = "distIndex", Type = "luaIndex", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		ClearCursor = {},
		GetAddOnCPUUsage = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "uiAddon", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "number", Nilable = false },
			},
		},
		GetAddOnMemoryUsage = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "uiAddon", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "number", Nilable = false },
			},
		},
		GetBuildInfo = {
			Returns = {
				{ Name = "buildVersion", Type = "cstring", Nilable = false },
				{ Name = "buildNumber", Type = "cstring", Nilable = false },
				{ Name = "buildDate", Type = "cstring", Nilable = false },
				{ Name = "interfaceVersion", Type = "number", Nilable = false },
				{ Name = "localizedVersion", Type = "cstring", Nilable = false },
				{ Name = "buildInfo", Type = "string", Nilable = false },
			},
		},
		GetCursorInfo = {},
		GetFramerate = {
			Returns = {
				{ Name = "framerate", Type = "number", Nilable = false },
			},
		},
		GetGameMessageInfo = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "gameErrorIndex", Type = "luaIndex", Nilable = false },
			},
			Returns = {
				{ Name = "errorName", Type = "cstring", Nilable = false },
				{ Name = "soundKitID", Type = "number", Nilable = true },
				{ Name = "voiceID", Type = "number", Nilable = true },
			},
		},
		GetNormalizedRealmName = {
			Returns = {
				{ Name = "result", Type = "cstring", Nilable = false },
			},
		},
		GetPhysicalScreenSize = {
			Returns = {
				{ Name = "sizeX", Type = "number", Nilable = false },
				{ Name = "sizeY", Type = "number", Nilable = false },
			},
		},
		GetRealZoneText = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "mapID", Type = "number", Nilable = true },
			},
			Returns = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		GetRealmName = {
			Returns = {
				{ Name = "realmName", Type = "cstring", Nilable = false },
			},
		},
		GetSubZoneText = {
			Returns = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		GetTime = {
			Returns = {
				{ Name = "time", Type = "number", Nilable = false },
			},
		},
		GetUnitSpeed = {
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitStatsRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "currentSpeed", Type = "number", Nilable = false },
				{ Name = "runSpeed", Type = "number", Nilable = false },
				{ Name = "flightSpeed", Type = "number", Nilable = false },
				{ Name = "swimSpeed", Type = "number", Nilable = false },
			},
		},
		IsControlKeyDown = {
			Returns = {
				{ Name = "down", Type = "bool", Nilable = false },
			},
		},
		IsInGuild = {
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		IsInInstance = {
			Returns = {
				{ Name = "isInInstance", Type = "bool", Nilable = false },
				{ Name = "instanceType", Type = "cstring", Nilable = false },
			},
		},
		IsMounted = {
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		IsResting = {
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		Screenshot = {},
		UnitCanAttack = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
				{ Name = "target", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitClass = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "className", Type = "cstring", Nilable = false, ConditionalSecret = true },
				{ Name = "classFilename", Type = "cstring", Nilable = false },
				{ Name = "classID", Type = "number", Nilable = false },
			},
		},
		UnitClassification = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "cstring", Nilable = false },
			},
		},
		UnitCreatureType = {
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "name", Type = "cstring", Nilable = false },
				{ Name = "id", Type = "number", Nilable = false },
			},
		},
		UnitExists = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitFactionGroup = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unitName", Type = "cstring", Nilable = false },
				{ Name = "checkDisplayRace", Type = "bool", Nilable = false, Default = false },
			},
			Returns = {
				{ Name = "factionGroupTag", Type = "cstring", Nilable = false },
				{ Name = "localized", Type = "cstring", Nilable = false },
			},
		},
		UnitFullName = {
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "unitName", Type = "cstring", Nilable = false },
				{ Name = "unitServer", Type = "cstring", Nilable = false },
			},
		},
		UnitGUID = {
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitTokenPvPRestrictedForAddOns", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "WOWGUID", Nilable = true },
			},
		},
		UnitIsConnected = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "isConnected", Type = "bool", Nilable = false },
			},
		},
		UnitIsPlayer = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = true },
				{ Name = "partyIndex", Type = "luaIndex", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitIsSameServer = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unitName", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitLevel = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "name", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "number", Nilable = false },
			},
		},
		UnitName = {
			SecretArguments = "AllowedWhenTainted",
			SecretWhenUnitNameIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "unitName", Type = "cstring", Nilable = false },
				{ Name = "unitServer", Type = "cstring", Nilable = false },
			},
		},
		UnitOnTaxi = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitPlayerControlled = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
		UnitRace = {
			MayReturnNothing = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "localizedRaceName", Type = "cstring", Nilable = false },
				{ Name = "englishRaceName", Type = "cstring", Nilable = false },
				{ Name = "raceID", Type = "number", Nilable = false },
			},
		},
		UnitSex = {
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenUnitIdentityRestricted = true,
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "sex", Type = "number", Nilable = true },
			},
		},
		UpdateAddOnCPUUsage = {},
		UpdateAddOnMemoryUsage = {},
		issecretvalue = {
			SecretArguments = "AllowedWhenUntainted",
			SecureHooksAllowed = false,
			Arguments = {
				{ Name = "value", Type = "LuaValueReference", Nilable = false },
			},
			Returns = {
				{ Name = "isSecret", Type = "bool", Nilable = false },
			},
		},
	},
	methods = {
		["DurationTextBindingObjectAPI:GetFontString"] = {
			Arguments = {},
			Returns = {
				{ Name = "fontString", Type = "SimpleFontString", Nilable = true },
			},
		},
		["DurationTextBindingObjectAPI:SetEnabled"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "enabled", Type = "bool", Nilable = false },
			},
		},
		["FrameAPICharacterModelBase:SetUnit"] = {
			RequiresDeclassifiedUnitIdentity = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
				{ Name = "blend", Type = "bool", Nilable = false, Default = true },
				{ Name = "useNativeForm", Type = "bool", Nilable = true },
			},
			Returns = {
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
		["FrameAPIModelSceneFrameActorBase:Hide"] = {
			Arguments = {},
		},
		["FrameAPIModelSceneFrameActorBase:IsShown"] = {
			Arguments = {},
			Returns = {
				{ Name = "isShown", Type = "bool", Nilable = false },
			},
		},
		["FrameAPIModelSceneFrameActorBase:SetAlpha"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Alpha },
			Arguments = {
				{ Name = "alpha", Type = "number", Nilable = false },
			},
		},
		["FrameAPIModelSceneFrameActorBase:SetScale"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scale", Type = "number", Nilable = false },
			},
		},
		["FrameAPIModelSceneFrameActorBase:SetShown"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "show", Type = "bool", Nilable = false, Default = false },
			},
		},
		["FrameAPIModelSceneFrameActorBase:Show"] = {
			Arguments = {},
		},
		["FrameAPISimpleCheckout:ClearFocus"] = {
			Arguments = {},
		},
		["FrameAPISimpleCheckout:SetFocus"] = {
			Arguments = {},
		},
		["FrameAPITooltip:SetText"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false, ConditionalSecret = true },
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "alpha", Type = "number", Nilable = false, ConditionalSecret = true, Default = 1 },
				{ Name = "wrap", Type = "bool", Nilable = false, ConditionalSecret = true, Default = false },
			},
		},
		["SimpleAnimAPI:HookScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = false },
				{ Name = "bindingType", Type = "ScriptBindingType", Nilable = false, Default = "Extrinsic" },
			},
			Returns = {
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
		["SimpleAnimAPI:SetScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = true },
			},
		},
		["SimpleAnimGroupAPI:HookScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = false },
				{ Name = "bindingType", Type = "ScriptBindingType", Nilable = false, Default = "Extrinsic" },
			},
			Returns = {
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
		["SimpleAnimGroupAPI:SetScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = true },
			},
		},
		["SimpleAnimScaleAPI:SetScale"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scaleX", Type = "number", Nilable = false },
				{ Name = "scaleY", Type = "number", Nilable = false },
			},
		},
		["SimpleBrowserAPI:ClearFocus"] = {
			Arguments = {},
		},
		["SimpleBrowserAPI:SetFocus"] = {
			Arguments = {},
		},
		["SimpleButtonAPI:GetFontString"] = {
			Arguments = {},
			Returns = {
				{ Name = "fontString", Type = "SimpleFontString", Nilable = false },
			},
		},
		["SimpleButtonAPI:GetText"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.Text },
			Arguments = {},
			Returns = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		["SimpleButtonAPI:RegisterForClicks"] = {
			IsProtectedFunction = true,
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "buttons", Type = "ClickButton", Nilable = false, StrideIndex = 1 },
			},
		},
		["SimpleButtonAPI:SetDisabledFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleButtonAPI:SetEnabled"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.ButtonState },
			Arguments = {
				{ Name = "enabled", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleButtonAPI:SetHighlightFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleButtonAPI:SetHighlightTexture"] = {
			CheckAllowChangeParent = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "asset", Type = "TextureAsset", Nilable = false },
				{ Name = "blendMode", Type = "BlendMode", Nilable = true },
			},
		},
		["SimpleButtonAPI:SetNormalFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleButtonAPI:SetText"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false, Default = "" },
			},
		},
		["SimpleEditBoxAPI:ClearFocus"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptedInput } },
			Arguments = {},
		},
		["SimpleEditBoxAPI:GetNumLetters"] = {
			MayReturnNothing = true,
			Arguments = {},
			Returns = {
				{ Name = "numLetters", Type = "number", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:GetText"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.Text },
			Arguments = {},
			Returns = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:HighlightText"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "start", Type = "number", Nilable = false, Default = 0 },
				{ Name = "stop", Type = "number", Nilable = false, Default = -1 },
			},
		},
		["SimpleEditBoxAPI:Insert"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetAutoFocus"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "autoFocus", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleEditBoxAPI:SetCursorPosition"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptedInput } },
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Cursor },
			Arguments = {
				{ Name = "cursorPosition", Type = "number", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetEnabled"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "enabled", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleEditBoxAPI:SetFocus"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptedInput } },
			Arguments = {},
		},
		["SimpleEditBoxAPI:SetFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetJustifyH"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "justifyH", Type = "JustifyHorizontal", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetMaxBytes"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "maxBytes", Type = "number", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetMaxLetters"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "maxLetters", Type = "number", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetMultiLine"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "multiline", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleEditBoxAPI:SetNumeric"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "isNumeric", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleEditBoxAPI:SetShadowOffset"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "offsetX", Type = "number", Nilable = false },
				{ Name = "offsetY", Type = "number", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetText"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		["SimpleEditBoxAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleEditBoxAPI:SetTextInsets"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "left", Type = "uiUnit", Nilable = false },
				{ Name = "right", Type = "uiUnit", Nilable = false },
				{ Name = "top", Type = "uiUnit", Nilable = false },
				{ Name = "bottom", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleFontAPI:SetAlpha"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "alpha", Type = "SingleColorValue", Nilable = false },
			},
		},
		["SimpleFontAPI:SetFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleFontAPI:SetJustifyH"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "justifyH", Type = "JustifyHorizontal", Nilable = false },
			},
		},
		["SimpleFontAPI:SetShadowOffset"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "offsetX", Type = "number", Nilable = false },
				{ Name = "offsetY", Type = "number", Nilable = false },
			},
		},
		["SimpleFontAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleFontStringAPI:GetStringHeight"] = {
			SecretWhenAnchoringSecret = true,
			Arguments = {},
			Returns = {
				{ Name = "height", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleFontStringAPI:GetStringWidth"] = {
			SecretWhenAnchoringSecret = true,
			Arguments = {},
			Returns = {
				{ Name = "width", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleFontStringAPI:GetText"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.Text },
			Arguments = {},
			Returns = {
				{ Name = "text", Type = "cstring", Nilable = false },
			},
		},
		["SimpleFontStringAPI:SetFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleFontStringAPI:SetJustifyH"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "justifyH", Type = "JustifyHorizontal", Nilable = false },
			},
		},
		["SimpleFontStringAPI:SetMaxLines"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "maxLines", Type = "number", Nilable = false },
			},
		},
		["SimpleFontStringAPI:SetShadowOffset"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "offsetX", Type = "number", Nilable = false },
				{ Name = "offsetY", Type = "number", Nilable = false },
			},
		},
		["SimpleFontStringAPI:SetText"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false, Default = "" },
			},
		},
		["SimpleFontStringAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.VertexColor, Enum.SecretAspect.Alpha },
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleFontStringAPI:SetWordWrap"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "wrap", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:CreateFontString"] = {
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "name", Type = "cstring", Nilable = true },
				{ Name = "drawLayer", Type = "DrawLayer", Nilable = true },
				{ Name = "templateName", Type = "cstring", Nilable = true },
			},
			Returns = {
				{ Name = "line", Type = "SimpleFontString", Nilable = false },
			},
		},
		["SimpleFrameAPI:CreateTexture"] = {
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "name", Type = "cstring", Nilable = true },
				{ Name = "drawLayer", Type = "DrawLayer", Nilable = true },
				{ Name = "templateName", Type = "cstring", Nilable = true },
				{ Name = "subLevel", Type = "number", Nilable = true },
			},
			Returns = {
				{ Name = "texture", Type = "SimpleTexture", Nilable = false },
			},
		},
		["SimpleFrameAPI:Hide"] = {
			IsProtectedFunction = true,
			Arguments = {},
		},
		["SimpleFrameAPI:IsShown"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.Shown },
			Arguments = {},
			Returns = {
				{ Name = "isShown", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:RegisterEvent"] = {
			AddsForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "eventName", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "registered", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:RegisterForDrag"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "buttons", Type = "MouseButton", Nilable = false, StrideIndex = 1 },
			},
		},
		["SimpleFrameAPI:RegisterUnitEvent"] = {
			AddsForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "eventName", Type = "cstring", Nilable = false },
				{ Name = "units", Type = "UnitTokenType", Nilable = false, StrideIndex = 1 },
			},
			Returns = {
				{ Name = "registered", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetAlpha"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Alpha },
			Arguments = {
				{ Name = "alpha", Type = "SingleColorValue", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetClipsChildren"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "clipsChildren", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetFrameLevel"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.FrameLevel },
			Arguments = {
				{ Name = "frameLevel", Type = "number", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetFrameStrata"] = {
			IsProtectedFunction = true,
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "strata", Type = "FrameStrata", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetIgnoreParentScale"] = {
			IsProtectedFunction = true,
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "ignore", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetMovable"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "movable", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetScale"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Scale },
			Arguments = {
				{ Name = "scale", Type = "number", Nilable = false },
			},
		},
		["SimpleFrameAPI:SetShown"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Shown },
			Arguments = {
				{ Name = "shown", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleFrameAPI:SetToplevel"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Toplevel },
			Arguments = {
				{ Name = "topLevel", Type = "bool", Nilable = false },
			},
		},
		["SimpleFrameAPI:Show"] = {
			IsProtectedFunction = true,
			Arguments = {},
		},
		["SimpleFrameAPI:UnregisterEvent"] = {
			AddsForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.EventRegistrations } },
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "eventName", Type = "cstring", Nilable = false },
			},
			Returns = {
				{ Name = "registered", Type = "bool", Nilable = false },
			},
		},
		["SimpleHTMLAPI:SetFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "textType", Type = "HTMLTextType", Nilable = false },
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleHTMLAPI:SetJustifyH"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "textType", Type = "HTMLTextType", Nilable = false },
				{ Name = "justifyH", Type = "JustifyHorizontal", Nilable = false },
			},
		},
		["SimpleHTMLAPI:SetShadowOffset"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "textType", Type = "HTMLTextType", Nilable = false },
				{ Name = "offsetX", Type = "number", Nilable = false },
				{ Name = "offsetY", Type = "number", Nilable = false },
			},
		},
		["SimpleHTMLAPI:SetText"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Text },
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false },
				{ Name = "ignoreMarkup", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleHTMLAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "textType", Type = "HTMLTextType", Nilable = false },
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleLineAPI:ClearAllPoints"] = {
			IsProtectedFunction = true,
			Arguments = {},
		},
		["SimpleMessageFrameAPI:AddMessage"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "text", Type = "cstring", Nilable = false },
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
				{ Name = "messageID", Type = "number", Nilable = true },
			},
		},
		["SimpleMessageFrameAPI:SetFontObject"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "font", Type = "SimpleFont", Nilable = false },
			},
		},
		["SimpleMessageFrameAPI:SetJustifyH"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "justifyH", Type = "JustifyHorizontal", Nilable = false },
			},
		},
		["SimpleMessageFrameAPI:SetShadowOffset"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "offsetX", Type = "number", Nilable = false },
				{ Name = "offsetY", Type = "number", Nilable = false },
			},
		},
		["SimpleMessageFrameAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleRegionAPI:SetAlpha"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Alpha },
			Arguments = {
				{ Name = "alpha", Type = "SingleColorValue", Nilable = false },
			},
		},
		["SimpleRegionAPI:SetIgnoreParentScale"] = {
			IsProtectedFunction = true,
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "ignore", Type = "bool", Nilable = false },
			},
		},
		["SimpleRegionAPI:SetScale"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Scale },
			Arguments = {
				{ Name = "scale", Type = "number", Nilable = false },
			},
		},
		["SimpleScriptRegionAPI:EnableMouse"] = {
			IsProtectedFunction = true,
			SecretArguments = "NotAllowed",
			Arguments = {
				{ Name = "enable", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleScriptRegionAPI:GetHeight"] = {
			ConstSecretAccessor = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretWhenAnchoringSecret = true,
			Arguments = {
				{ Name = "ignoreRect", Type = "bool", Nilable = false, Default = false },
			},
			Returns = {
				{ Name = "height", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScriptRegionAPI:Hide"] = {
			Arguments = {},
		},
		["SimpleScriptRegionAPI:HookScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = false },
				{ Name = "bindingType", Type = "ScriptBindingType", Nilable = false, Default = "Extrinsic" },
			},
			Returns = {
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
		["SimpleScriptRegionAPI:IsShown"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.Shown },
			Arguments = {},
			Returns = {
				{ Name = "isShown", Type = "bool", Nilable = false },
			},
		},
		["SimpleScriptRegionAPI:SetScript"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.ScriptBindings } },
			RequiresAssignableScript = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scriptTypeName", Type = "ScriptTypeName", Nilable = false },
				{ Name = "script", Type = "LuaFunctionReference", Nilable = true },
			},
		},
		["SimpleScriptRegionAPI:SetShown"] = {
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.Shown },
			Arguments = {
				{ Name = "show", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleScriptRegionAPI:Show"] = {
			Arguments = {},
		},
		["SimpleScriptRegionResizingAPI:ClearAllPoints"] = {
			IsProtectedFunction = true,
			Arguments = {},
		},
		["SimpleScriptRegionResizingAPI:SetAllPoints"] = {
			CheckAllowInheritForbiddenLayoutAspects = true,
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "relativeTo", Type = "ScriptRegion", Nilable = false },
				{ Name = "doResize", Type = "bool", Nilable = false, Default = true },
			},
		},
		["SimpleScriptRegionResizingAPI:SetHeight"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "height", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScriptRegionResizingAPI:SetPoint"] = {
			CheckAllowInheritForbiddenLayoutAspects = true,
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "point", Type = "FramePoint", Nilable = false },
				{ Name = "relativeTo", Type = "ScriptRegion", Nilable = false },
				{ Name = "relativePoint", Type = "FramePoint", Nilable = false },
				{ Name = "offsetX", Type = "uiUnit", Nilable = false },
				{ Name = "offsetY", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScriptRegionResizingAPI:SetSize"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "x", Type = "uiUnit", Nilable = false },
				{ Name = "y", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScriptRegionResizingAPI:SetWidth"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "width", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScrollFrameAPI:GetVerticalScroll"] = {
			SecretReturnsForAspect = { Enum.SecretAspect.ScrollOffset },
			Arguments = {},
			Returns = {
				{ Name = "offset", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleScrollFrameAPI:SetScrollChild"] = {
			CheckAllowChangeParent = true,
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "scrollChild", Type = "SimpleFrame", Nilable = false },
			},
		},
		["SimpleScrollFrameAPI:SetVerticalScroll"] = {
			IsProtectedFunction = true,
			SecretArguments = "AllowedWhenUntainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.ScrollOffset },
			Arguments = {
				{ Name = "offset", Type = "uiUnit", Nilable = false },
			},
		},
		["SimpleSliderAPI:SetEnabled"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "enabled", Type = "bool", Nilable = false },
			},
		},
		["SimpleTextureBaseAPI:SetAtlas"] = {
			SecretArguments = "AllowedWhenTainted",
			Arguments = {
				{ Name = "atlas", Type = "textureAtlas", Nilable = false },
				{ Name = "useAtlasSize", Type = "bool", Nilable = false, NeverSecret = true, Default = false },
				{ Name = "filterMode", Type = "FilterMode", Nilable = true, NeverSecret = true },
				{ Name = "resetTexCoords", Type = "bool", Nilable = true, NeverSecret = true },
				{ Name = "wrapModeHorizontal", Type = "cstring", Nilable = true, NeverSecret = true },
				{ Name = "wrapModeVertical", Type = "cstring", Nilable = true, NeverSecret = true },
			},
		},
		["SimpleTextureBaseAPI:SetColorTexture"] = {
			ChecksForbiddenAspects = { { Argument = "self", Aspect = Enum.ForbiddenAspect.SetTexture } },
			SecretArguments = "AllowedWhenTainted",
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
			},
		},
		["SimpleTextureBaseAPI:SetSnapToPixelGrid"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "snap", Type = "bool", Nilable = false, Default = false },
			},
		},
		["SimpleTextureBaseAPI:SetTexCoord"] = {
			SecretArguments = "AllowedWhenTainted",
			SecretArgumentsAddAspect = { Enum.SecretAspect.TexCoords },
			Arguments = {
				{ Name = "left", Type = "number", Nilable = false },
				{ Name = "right", Type = "number", Nilable = false },
				{ Name = "bottom", Type = "number", Nilable = false },
				{ Name = "top", Type = "number", Nilable = false },
			},
		},
		["SimpleTextureBaseAPI:SetTexture"] = {
			SecretArguments = "AllowedWhenTainted",
			Arguments = {
				{ Name = "textureAsset", Type = "cstring", Nilable = true },
				{ Name = "wrapModeHorizontal", Type = "cstring", Nilable = true },
				{ Name = "wrapModeVertical", Type = "cstring", Nilable = true },
				{ Name = "filterMode", Type = "cstring", Nilable = true },
			},
			Returns = {
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
	},
	events = {
		ADDON_LOADED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "addOnName", Type = "cstring", Nilable = false },
				{ Name = "containsBindings", Type = "bool", Nilable = false },
			},
		},
		BAG_UPDATE_DELAYED = {
			UniqueEvent = true,
		},
		CHAT_MSG_ADDON = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "prefix", Type = "cstring", Nilable = false },
				{ Name = "text", Type = "cstring", Nilable = false },
				{ Name = "channel", Type = "cstring", Nilable = false },
				{ Name = "sender", Type = "cstring", Nilable = false },
				{ Name = "target", Type = "cstring", Nilable = false },
				{ Name = "zoneChannelID", Type = "number", Nilable = false },
				{ Name = "localID", Type = "number", Nilable = false },
				{ Name = "name", Type = "cstring", Nilable = false },
				{ Name = "instanceID", Type = "number", Nilable = false },
			},
		},
		CHAT_MSG_ADDON_LOGGED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "prefix", Type = "cstring", Nilable = false },
				{ Name = "text", Type = "cstring", Nilable = false },
				{ Name = "channel", Type = "cstring", Nilable = false },
				{ Name = "sender", Type = "cstring", Nilable = false },
				{ Name = "target", Type = "cstring", Nilable = false },
				{ Name = "zoneChannelID", Type = "number", Nilable = false },
				{ Name = "localID", Type = "number", Nilable = false },
				{ Name = "name", Type = "cstring", Nilable = false },
				{ Name = "instanceID", Type = "number", Nilable = false },
			},
		},
		ENCOUNTER_END = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "encounterID", Type = "number", Nilable = false },
				{ Name = "encounterName", Type = "cstring", Nilable = false },
				{ Name = "difficultyID", Type = "number", Nilable = false },
				{ Name = "groupSize", Type = "number", Nilable = false },
				{ Name = "success", Type = "number", Nilable = false },
				{ Name = "encounterUnitStatus", Type = "table", InnerType = "EncounterUnitStatus", Nilable = false },
			},
		},
		GET_ITEM_INFO_RECEIVED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "itemID", Type = "number", Nilable = false },
				{ Name = "success", Type = "bool", Nilable = false },
			},
		},
		GOSSIP_SHOW = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "uiTextureKit", Type = "textureKit", Nilable = true },
			},
		},
		GROUP_ROSTER_UPDATE = {
			UniqueEvent = true,
		},
		GUILD_ROSTER_UPDATE = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "canRequestRosterUpdate", Type = "bool", Nilable = false },
			},
		},
		ITEM_TEXT_READY = {
			SynchronousEvent = true,
		},
		MAJOR_FACTION_RENOWN_LEVEL_CHANGED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "majorFactionID", Type = "number", Nilable = false },
				{ Name = "newRenownLevel", Type = "number", Nilable = false },
				{ Name = "oldRenownLevel", Type = "number", Nilable = false },
			},
		},
		NAME_PLATE_UNIT_ADDED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "unitToken", Type = "UnitTokenType", Nilable = false },
			},
		},
		PARTY_KILL = {
			SecretWhenUnitIdentityRestricted = true,
			SynchronousEvent = true,
			Payload = {
				{ Name = "attackerGUID", Type = "WOWGUID", Nilable = false },
				{ Name = "targetGUID", Type = "WOWGUID", Nilable = false },
			},
		},
		PLAYER_CAMPING = {
			SynchronousEvent = true,
		},
		PLAYER_DEAD = {
			SynchronousEvent = true,
		},
		PLAYER_ENTERING_WORLD = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "isInitialLogin", Type = "bool", Nilable = false },
				{ Name = "isReloadingUi", Type = "bool", Nilable = false },
			},
		},
		PLAYER_EQUIPMENT_CHANGED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "equipmentSlot", Type = "number", Nilable = false },
				{ Name = "hasCurrent", Type = "bool", Nilable = false },
			},
		},
		PLAYER_GUILD_UPDATE = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "unitTarget", Type = "UnitTokenVariant", Nilable = false },
			},
		},
		PLAYER_LEVEL_UP = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "level", Type = "number", Nilable = false },
				{ Name = "healthDelta", Type = "number", Nilable = false },
				{ Name = "powerDelta", Type = "number", Nilable = false },
				{ Name = "numNewTalents", Type = "number", Nilable = false },
				{ Name = "numNewPvpTalentSlots", Type = "number", Nilable = false },
				{ Name = "strengthDelta", Type = "number", Nilable = false },
				{ Name = "agilityDelta", Type = "number", Nilable = false },
				{ Name = "staminaDelta", Type = "number", Nilable = false },
				{ Name = "intellectDelta", Type = "number", Nilable = false },
			},
		},
		PLAYER_LOGIN = {
			SynchronousEvent = true,
		},
		PLAYER_LOGOUT = {
			SynchronousEvent = true,
		},
		PLAYER_MOUNT_DISPLAY_CHANGED = {
			SynchronousEvent = true,
		},
		PLAYER_QUITING = {
			SynchronousEvent = true,
		},
		PLAYER_REGEN_DISABLED = {
			SynchronousEvent = true,
		},
		PLAYER_REGEN_ENABLED = {
			SynchronousEvent = true,
		},
		PLAYER_TARGET_CHANGED = {
			SynchronousEvent = true,
		},
		PLAYER_UPDATE_RESTING = {
			SynchronousEvent = true,
		},
		QUEST_ACCEPTED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "questId", Type = "number", Nilable = false },
			},
		},
		QUEST_COMPLETE = {
			SynchronousEvent = true,
		},
		QUEST_DETAIL = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "questStartItemID", Type = "number", Nilable = true },
			},
		},
		QUEST_GREETING = {
			SynchronousEvent = true,
		},
		QUEST_PROGRESS = {
			SynchronousEvent = true,
		},
		QUEST_TURNED_IN = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "questID", Type = "number", Nilable = false },
				{ Name = "xpReward", Type = "number", Nilable = false },
				{ Name = "moneyReward", Type = "number", Nilable = false },
			},
		},
		QUEST_WATCH_UPDATE = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "questID", Type = "number", Nilable = false },
			},
		},
		SCREENSHOT_FAILED = {
			SynchronousEvent = true,
		},
		SCREENSHOT_SUCCEEDED = {
			SynchronousEvent = true,
		},
		TRADE_ACCEPT_UPDATE = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "playerAccepted", Type = "number", Nilable = false },
				{ Name = "targetAccepted", Type = "number", Nilable = false },
			},
		},
		TRADE_CLOSED = {
			SynchronousEvent = true,
		},
		TRADE_MONEY_CHANGED = {
			SynchronousEvent = true,
		},
		TRADE_PLAYER_ITEM_CHANGED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "tradeSlotIndex", Type = "number", Nilable = false },
			},
		},
		TRADE_SHOW = {
			SynchronousEvent = true,
		},
		TRADE_TARGET_ITEM_CHANGED = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "tradeSlotIndex", Type = "number", Nilable = false },
			},
		},
		UI_INFO_MESSAGE = {
			UniqueEvent = true,
			Payload = {
				{ Name = "errorType", Type = "luaIndex", Nilable = false },
				{ Name = "message", Type = "string", Nilable = false },
			},
		},
		UNIT_AURA = {
			SecretWhenAurasRestricted = true,
			SynchronousEvent = true,
			Payload = {
				{ Name = "unitTarget", Type = "UnitTokenVariant", Nilable = false },
				{ Name = "updateInfo", Type = "UnitAuraUpdateInfo", Nilable = false },
			},
		},
		UPDATE_BATTLEFIELD_STATUS = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "battleFieldIndex", Type = "number", Nilable = false },
			},
		},
		UPDATE_MOUSEOVER_UNIT = {
			SynchronousEvent = true,
		},
		ZONE_CHANGED = {
			SynchronousEvent = true,
		},
		ZONE_CHANGED_INDOORS = {
			SynchronousEvent = true,
		},
		ZONE_CHANGED_NEW_AREA = {
			SynchronousEvent = true,
		},
	},
	undocumented = {
		"CreateFrame",
		"GetBattlefieldWinner",
		"GetGreetingText",
		"GetGuildRosterInfo",
		"GetInventoryItemLink",
		"GetNumGuildMembers",
		"GetObjectiveText",
		"GetPlayerTradeMoney",
		"GetProgressText",
		"GetQuestText",
		"GetRewardText",
		"GetTargetTradeMoney",
		"GetTitleText",
		"GetTradePlayerItemInfo",
		"GetTradeTargetItemInfo",
		"InCombatLockdown",
		"IsInGroup",
		"IsInRaid",
		"ItemTextGetCreator",
		"ItemTextGetItem",
		"ItemTextGetText",
		"PlaySound",
		"bit.band",
		"bit.bnot",
		"bit.bor",
		"bit.bxor",
		"bit.lshift",
		"bit.rshift",
		"date",
		"hooksecurefunc",
		"strtrim",
		"time",
	},
}
