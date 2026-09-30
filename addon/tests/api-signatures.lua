-- The documented WoW Forever 1.60.1.70009 API that Timeways uses.
-- Written by scripts/wow-api.sh from Blizzard_APIDocumentationGenerated. Do not edit.
-- A patch can change the arguments, returns, or secret flags and keep the name. The diff shows it.
-- The scan does not know the type of each object, so methods has each widget type with a called name.
return {
	build = "1.60.1.70009",
	functions = {
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
		["C_GossipInfo.GetText"] = {
			Returns = {
				{ Name = "gossipText", Type = "cstring", Nilable = false },
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
		UnitClassification = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = false },
			},
			Returns = {
				{ Name = "result", Type = "cstring", Nilable = false },
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
		UnitPlayerControlled = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "unit", Type = "UnitToken", Nilable = true },
			},
			Returns = {
				{ Name = "result", Type = "bool", Nilable = false },
			},
		},
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
		["FrameAPIModelSceneFrameActorBase:Hide"] = {
			Arguments = {},
		},
		["FrameAPIModelSceneFrameActorBase:IsShown"] = {
			Arguments = {},
			Returns = {
				{ Name = "isShown", Type = "bool", Nilable = false },
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
		["SimpleEditBoxAPI:SetAutoFocus"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "autoFocus", Type = "bool", Nilable = false, Default = false },
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
		["SimpleMessageFrameAPI:SetTextColor"] = {
			SecretArguments = "AllowedWhenUntainted",
			Arguments = {
				{ Name = "colorR", Type = "number", Nilable = false },
				{ Name = "colorG", Type = "number", Nilable = false },
				{ Name = "colorB", Type = "number", Nilable = false },
				{ Name = "a", Type = "SingleColorValue", Nilable = true },
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
		GOSSIP_SHOW = {
			SynchronousEvent = true,
			Payload = {
				{ Name = "uiTextureKit", Type = "textureKit", Nilable = true },
			},
		},
		ITEM_TEXT_READY = {
			SynchronousEvent = true,
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
		PLAYER_TARGET_CHANGED = {
			SynchronousEvent = true,
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
		SCREENSHOT_FAILED = {
			SynchronousEvent = true,
		},
		SCREENSHOT_SUCCEEDED = {
			SynchronousEvent = true,
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
		"GetGreetingText",
		"GetObjectiveText",
		"GetProgressText",
		"GetQuestText",
		"GetRewardText",
		"GetTitleText",
		"InCombatLockdown",
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
