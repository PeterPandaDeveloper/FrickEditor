using System;
using System.IO;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using UndertaleModLib.Util;
using UndertaleModLib.Models;
using ImageMagick;

EnsureDataLoaded();

// ==============================================================
// 1. PREPARACIÓN Y PRE-COMPOSICIÓN DE ASSETS (V23)
// ==============================================================
string importFolder = @"D:\Frickbears-mod\Murray_Sprites_To_Import";
string assetsFolder = @"D:\Frickbears-mod\Murray_Assets";
string animFolder   = @"D:\Frickbears-mod\MurrayMooseAnimation";

if (Directory.Exists(importFolder)) Directory.Delete(importFolder, true);
Directory.CreateDirectory(importFolder);

// Assets base (Retratos, Gafas, Cabeza, Tiles)
string[] baseAssets = new string[] {
    "sMurray_head.png", "sMurray_head_0.png",
    "sMurray_head_Scare.png", "sMurray_head_Scare_0.png",
    "sMurray_head_Alt.png", "sMurray_head_Alt_0.png",
    "sMurray_Glasses.png", "sMurray_Glasses_0.png",
    "sMurray_Flash.png", "sMurray_Flash_0.png",
    "sTile_Murray_0.png", "sTile_Murray_0.png",
    "sTile_Murray_1.png", "sTile_Murray_1.png",
    "sTile_Murray_2.png", "sTile_Murray_2.png"
};

for (int i = 0; i < baseAssets.Length; i += 2)
{
    string src = Path.Combine(assetsFolder, baseAssets[i]);
    string dst = Path.Combine(importFolder, baseAssets[i + 1]);
    if (File.Exists(src)) File.Copy(src, dst, true);
}

// NUEVA ANIMACIÓN DE CUERPO (sMurray_body): Frames 000.png a 005.png (392x228)
if (Directory.Exists(animFolder))
{
    for (int f = 0; f <= 5; f++)
    {
        string frameSrc = Path.Combine(animFolder, $"{f:000}.png");
        string frameDst = Path.Combine(importFolder, $"sMurray_body_{f}.png");
        if (File.Exists(frameSrc))
        {
            File.Copy(frameSrc, frameDst, true);
        }
    }
}
else
{
    string srcBody = Path.Combine(assetsFolder, "sMurray_body.png");
    if (File.Exists(srcBody)) File.Copy(srcBody, Path.Combine(importFolder, "sMurray_body_0.png"), true);
}

// PRE-COMPONER JUMPSCARE EN 3 FRAMES COMPLETOS DE 640x640
string jBodyPath = Path.Combine(assetsFolder, "sJumpscare_Murray_0.png");
string jHead1Path = Path.Combine(assetsFolder, "sJumpscare_Murray_1.png");
string jHead2Path = Path.Combine(assetsFolder, "sJumpscare_Murray_2.png");

if (File.Exists(jBodyPath) && File.Exists(jHead1Path) && File.Exists(jHead2Path))
{
    try
    {
        using MagickImage bodyImg = new(jBodyPath);
        using MagickImage head1Img = new(jHead1Path);
        using MagickImage head2Img = new(jHead2Path);

        // Frame 0: Cuerpo + Cabeza 1
        using (MagickImage frame0 = new(MagickColors.Transparent, (uint)640, (uint)640))
        {
            frame0.Composite(bodyImg, 0, 0, CompositeOperator.Over);
            frame0.Composite(head1Img, 0, 0, CompositeOperator.Over);
            frame0.Write(Path.Combine(importFolder, "sMurray_Jumpscare_0.png"));
        }
        // Frame 1: Cuerpo + Cabeza 2 (Mandíbula mordiendo abajo)
        using (MagickImage frame1 = new(MagickColors.Transparent, (uint)640, (uint)640))
        {
            frame1.Composite(bodyImg, 0, 10, CompositeOperator.Over);
            frame1.Composite(head2Img, 0, 10, CompositeOperator.Over);
            frame1.Write(Path.Combine(importFolder, "sMurray_Jumpscare_1.png"));
        }
        // Frame 2: Cuerpo + Cabeza 2 (Mandíbula abierta arriba)
        using (MagickImage frame2 = new(MagickColors.Transparent, (uint)640, (uint)640))
        {
            frame2.Composite(bodyImg, 0, 4, CompositeOperator.Over);
            frame2.Composite(head2Img, 0, -6, CompositeOperator.Over);
            frame2.Write(Path.Combine(importFolder, "sMurray_Jumpscare_2.png"));
        }
    }
    catch { }
}

// ==============================================================
// 2. EMPAQUETADO LIMPIO EN ATLAS (V23)
// ==============================================================
Regex sprFrameRegex = new(@"^(.+?)(?:_(\d+))$", RegexOptions.Compiled);
string packDir = Path.Join(ExePath, "Packager");
Directory.CreateDirectory(packDir);
string outName = Path.Join(packDir, "atlas_murray_v25.txt");

Packer packer = new Packer();
packer.Process(importFolder, "*.png", 2048, 2, false);
packer.SaveAtlasses(outName);

int lastTextPage = Data.EmbeddedTextures.Count - 1;
int lastTextPageItem = Data.TexturePageItems.Count - 1;
string prefix = Path.Join(Path.GetDirectoryName(outName), Path.GetFileNameWithoutExtension(outName));
int atlasCount = 0;

foreach (Atlas atlas in packer.Atlasses)
{
    string atlasName = $"{prefix}{atlasCount:000}.png";
    using MagickImage atlasImage = TextureWorker.ReadBGRAImageFromFile(atlasName);

    UndertaleEmbeddedTexture texture = new();
    texture.Name = new UndertaleString($"Texture {++lastTextPage}");
    texture.TextureData.Image = GMImage.FromMagickImage(atlasImage).ConvertToPng();
    Data.EmbeddedTextures.Add(texture);

    foreach (Node n in atlas.Nodes)
    {
        if (n.Texture != null)
        {
            UndertaleTexturePageItem texturePageItem = new();
            texturePageItem.Name = new UndertaleString($"PageItem {++lastTextPageItem}");
            texturePageItem.SourceX = (ushort)n.Bounds.X;
            texturePageItem.SourceY = (ushort)n.Bounds.Y;
            texturePageItem.SourceWidth = (ushort)n.Bounds.Width;
            texturePageItem.SourceHeight = (ushort)n.Bounds.Height;
            texturePageItem.TargetX = (ushort)n.Texture.TargetX;
            texturePageItem.TargetY = (ushort)n.Texture.TargetY;
            texturePageItem.TargetWidth = (ushort)n.Bounds.Width;
            texturePageItem.TargetHeight = (ushort)n.Bounds.Height;
            texturePageItem.BoundingWidth = (ushort)n.Texture.BoundingWidth;
            texturePageItem.BoundingHeight = (ushort)n.Texture.BoundingHeight;
            texturePageItem.TexturePage = texture;
            Data.TexturePageItems.Add(texturePageItem);

            string stripped = Path.GetFileNameWithoutExtension(n.Texture.Source);
            string spriteName = stripped;
            int frame = 0;
            try
            {
                var spriteParts = sprFrameRegex.Match(stripped);
                if (spriteParts.Success)
                {
                    spriteName = spriteParts.Groups[1].Value;
                    int.TryParse(spriteParts.Groups[2].Value, out frame);
                }
            }
            catch { }

            UndertaleSprite.TextureEntry texentry = new();
            texentry.Texture = texturePageItem;

            UndertaleSprite sprite = Data.Sprites.ByName(spriteName);
            if (sprite is null)
            {
                UndertaleSprite newSprite = new();
                newSprite.Name = Data.Strings.MakeString(spriteName);
                newSprite.Width = (uint)n.Texture.BoundingWidth;
                newSprite.Height = (uint)n.Texture.BoundingHeight;
                newSprite.MarginLeft = n.Texture.TargetX;
                newSprite.MarginRight = n.Texture.TargetX + n.Bounds.Width - 1;
                newSprite.MarginTop = n.Texture.TargetY;
                newSprite.MarginBottom = n.Texture.TargetY + n.Bounds.Height - 1;
                newSprite.OriginX = 0;
                newSprite.OriginY = 0;
                if (frame > 0)
                {
                    for (int f = 0; f < frame; f++) newSprite.Textures.Add(null);
                }
                newSprite.Textures.Add(texentry);
                Data.Sprites.Add(newSprite);
            }
            else
            {
                sprite.Width = (uint)n.Texture.BoundingWidth;
                sprite.Height = (uint)n.Texture.BoundingHeight;
                sprite.MarginLeft = n.Texture.TargetX;
                sprite.MarginRight = n.Texture.TargetX + n.Bounds.Width - 1;
                sprite.MarginTop = n.Texture.TargetY;
                sprite.MarginBottom = n.Texture.TargetY + n.Bounds.Height - 1;
                sprite.OriginX = 0;
                sprite.OriginY = 0;
                while (frame >= sprite.Textures.Count)
                {
                    sprite.Textures.Add(texentry);
                }
                sprite.Textures[frame] = texentry;
            }
        }
    }
    atlasCount++;
}

// ==============================================================
// 3. OBJETOS DEL MOD
// ==============================================================
UndertaleGameObject objMurray = Data.GameObjects.ByName("oMurray_BackWindow");
if (objMurray == null)
{
    objMurray = new UndertaleGameObject()
    {
        Name = Data.Strings.MakeString("oMurray_BackWindow"),
        Visible = true,
        Persistent = false
    };
    Data.GameObjects.Add(objMurray);
}

UndertaleGameObject objDeath = Data.GameObjects.ByName("oMurray_DeathScreen");
if (objDeath == null)
{
    objDeath = new UndertaleGameObject()
    {
        Name = Data.Strings.MakeString("oMurray_DeathScreen"),
        Visible = true,
        Persistent = true
    };
    Data.GameObjects.Add(objDeath);
}

UndertaleGameObject objCNTile = Data.GameObjects.ByName("oMurray_CustomNightTile");
if (objCNTile == null)
{
    objCNTile = new UndertaleGameObject()
    {
        Name = Data.Strings.MakeString("oMurray_CustomNightTile"),
        Visible = true,
        Persistent = false
    };
    Data.GameObjects.Add(objCNTile);
}

// ==============================================================
// 4. COMPILAR Y VINCULAR CÓDIGO GML (V23)
// ==============================================================
UndertaleModLib.Compiler.CodeImportGroup importGroup = new(Data)
{
    AutoCreateAssets = true,
    MainThreadAction = MainThreadAction
};

string codeDir = @"D:\Frickbears-mod\Murray_GML_Code\";

importGroup.QueueReplace(objMurray.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_BackWindow_Create_0.gml")));
importGroup.QueueReplace(objMurray.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_BackWindow_Step_0.gml")));
importGroup.QueueReplace(objMurray.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_BackWindow_Draw_0.gml")));
importGroup.QueueReplace(objMurray.EventHandlerFor(EventType.Draw, EventSubtypeDraw.DrawGUI, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_BackWindow_Draw_64.gml")));
importGroup.QueueReplace(objMurray.EventHandlerFor(EventType.CleanUp, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_BackWindow_CleanUp_0.gml")));

importGroup.QueueReplace(objDeath.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_DeathScreen_Step_0.gml")));
importGroup.QueueReplace(objDeath.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_DeathScreen_Draw_0.gml")));

importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Create, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_CustomNightTile_Create_0.gml")));
importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Step, EventSubtypeStep.Step, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_CustomNightTile_Step_0.gml")));
importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.Draw, EventSubtypeDraw.Draw, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_CustomNightTile_Draw_0.gml")));
importGroup.QueueReplace(objCNTile.EventHandlerFor(EventType.CleanUp, Data), File.ReadAllText(Path.Combine(codeDir, "gml_Object_oMurray_CustomNightTile_CleanUp_0.gml")));

// ==============================================================
// 5. INYECCIÓN LIMPIA EN SALAS
// ==============================================================
void AddInstanceToRoom(string roomName, UndertaleGameObject targetObj, int targetX, int targetY)
{
    UndertaleRoom rm = Data.Rooms.ByName(roomName);
    if (rm != null)
    {
        UndertaleRoom.Layer targetLayer = null;
        foreach (var l in rm.Layers)
        {
            if (l.LayerType == UndertaleRoom.LayerType.Instances && l.InstancesData != null)
            {
                targetLayer = l;
                break;
            }
        }
        
        if (targetLayer != null)
        {
            bool alreadyThere = false;
            foreach (var inst in targetLayer.InstancesData.Instances)
            {
                if (inst.ObjectDefinition == targetObj)
                {
                    alreadyThere = true;
                    break;
                }
            }
            if (!alreadyThere)
            {
                var newInst = new UndertaleRoom.GameObject()
                {
                    InstanceID = Data.GeneralInfo.LastObj++,
                    ObjectDefinition = targetObj,
                    X = targetX,
                    Y = targetY,
                    ScaleX = 1,
                    ScaleY = 1,
                    Color = 0xFFFFFFFF
                };
                targetLayer.InstancesData.Instances.Add(newInst);
            }
        }
        
        bool inGameObjects = false;
        foreach (var go in rm.GameObjects)
        {
            if (go.ObjectDefinition == targetObj)
            {
                inGameObjects = true;
                break;
            }
        }
        if (!inGameObjects)
        {
            rm.GameObjects.Add(new UndertaleRoom.GameObject()
            {
                InstanceID = Data.GeneralInfo.LastObj++,
                ObjectDefinition = targetObj,
                X = targetX,
                Y = targetY,
                ScaleX = 1,
                ScaleY = 1,
                Color = 0xFFFFFFFF
            });
        }
    }
}

AddInstanceToRoom("Office", objMurray, 0, 0);
AddInstanceToRoom("Fredbear_Office", objMurray, 0, 0);
AddInstanceToRoom("CustomNightMenu", objCNTile, 0, 0);
AddInstanceToRoom("EndlessShopSelection_FinalNight", objCNTile, 0, 0);

// ==============================================================
// 6. PANTALLA DE GAME OVER (TIME OF DEATH Y RETRATO CARMESÍ)
// ==============================================================
var goCreate = Data.Code.ByName("gml_Object_oGameOver_Create_0");
if (goCreate != null)
{
    try
    {
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goCreate, Data.ToolInfo.DecompilerSettings).DecompileToString();
        if (!decomp.Contains("Murray Moose"))
        {
            string target = "Tip = \"You passed out.\";";
            string rep = @"if (variable_global_exists(""MurrayDeath"") && global.MurrayDeath)
{
    Tip = ""Murray Moose: Flashing him only works after you remove his sunglasses."";
    if (variable_global_exists(""Murray_SavedTimeSeconds"") && global.Murray_SavedTimeSeconds > 0)
    {
        global.TimeOfDeath = global.Murray_SavedTimeSeconds;
    }
    audio_stop_all();
    var mus_go = asset_get_index(""musTitleScreen_Evil"");
    if (mus_go == -1) mus_go = asset_get_index(""sfxSalvageStinger"");
    if (mus_go == -1) mus_go = asset_get_index(""sfxMusicBoxStinger"");
    if (mus_go != -1) audio_play_sound(mus_go, 10, false);
}
else
{
    Tip = ""You passed out."";
}";
            if (decomp.Contains(target))
            {
                string patched = decomp.Replace(target, rep);
                importGroup.QueueReplace(goCreate, patched);
            }
        }
        else if (!decomp.Contains("global.TimeOfDeath"))
        {
            string target = "Tip = \"Murray Moose: Flashing him only works after you remove his sunglasses.\";";
            string rep = @"Tip = ""Murray Moose: Flashing him only works after you remove his sunglasses."";
    if (variable_global_exists(""Murray_SavedTimeSeconds"") && global.Murray_SavedTimeSeconds > 0)
    {
        global.TimeOfDeath = global.Murray_SavedTimeSeconds;
    }";
            if (decomp.Contains(target))
            {
                string patched = decomp.Replace(target, rep);
                importGroup.QueueReplace(goCreate, patched);
            }
        }
    }
    catch { }
}

var goDraw = Data.Code.ByName("gml_Object_oGameOver_Draw_0");
if (goDraw != null)
{
    try
    {
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), goDraw, Data.ToolInfo.DecompilerSettings).DecompileToString();
        if (!decomp.Contains("MurrayDeath"))
        {
            string target = "if (global.Jumpscare == -1)";
            string rep = @"if (variable_global_exists(""MurrayDeath"") && global.MurrayDeath)
{
    if (variable_global_exists(""Murray_SavedTimeSeconds"") && global.Murray_SavedTimeSeconds > 0)
    {
        global.TimeOfDeath = global.Murray_SavedTimeSeconds;
    }
    var spr_alt = asset_get_index(""sMurray_head_Alt"");
    if (spr_alt == -1) spr_alt = asset_get_index(""sMurray_head"");
    if (spr_alt != -1)
    {
        var scale = 3.2;
        var spr_w = sprite_get_width(spr_alt) * scale;
        var spr_h = sprite_get_height(spr_alt) * scale;
        var hx = (640 - spr_w) / 2;
        var hy = ((360 - spr_h) / 2) - 10;
        draw_sprite_ext(spr_alt, 0, hx, hy, scale, scale, 0, make_color_rgb(175, 20, 20), 0.9);
    }
}
else if (global.Jumpscare == -1)";
            if (decomp.Contains(target))
            {
                string patched = decomp.Replace(target, rep);
                importGroup.QueueReplace(goDraw, patched);
            }
        }
        else if (!decomp.Contains("global.TimeOfDeath"))
        {
            string target = "if (variable_global_exists(\"MurrayDeath\") && global.MurrayDeath)\n{";
            string rep = "if (variable_global_exists(\"MurrayDeath\") && global.MurrayDeath)\n{\n    if (variable_global_exists(\"Murray_SavedTimeSeconds\") && global.Murray_SavedTimeSeconds > 0)\n    {\n        global.TimeOfDeath = global.Murray_SavedTimeSeconds;\n    }";
            if (decomp.Contains(target))
            {
                string patched = decomp.Replace(target, rep);
                importGroup.QueueReplace(goDraw, patched);
            }
        }
    }
    catch { }
}

// ==============================================================
// 7. COMPATIBILIDAD TOTAL SUN & MOON (DESACOPLAR PERSIANA DERECHA)
// ==============================================================
void PatchSunMoonCode(UndertaleCode codeEntry)
{
    if (codeEntry == null) return;
    try
    {
        string decomp = new Underanalyzer.Decompiler.DecompileContext(new UndertaleModLib.Decompiler.GlobalDecompileContext(Data), codeEntry, Data.ToolInfo.DecompilerSettings).DecompileToString();
        bool modified = false;

        if (decomp.Contains("with (oWindowBlinds)"))
        {
            decomp = decomp.Replace("with (oWindowBlinds)", "with (instance_nearest(1226, 1085, oWindowBlinds))");
            modified = true;
        }
        if (decomp.Contains("oWindowBlinds.Open"))
        {
            decomp = decomp.Replace("oWindowBlinds.Open", "instance_nearest(1226, 1085, oWindowBlinds).Open");
            modified = true;
        }
        if (decomp.Contains("oWindowBlinds.ForcedOpen"))
        {
            decomp = decomp.Replace("oWindowBlinds.ForcedOpen", "instance_nearest(1226, 1085, oWindowBlinds).ForcedOpen");
            modified = true;
        }

        if (modified)
        {
            importGroup.QueueReplace(codeEntry, decomp);
        }
    }
    catch { }
}

PatchSunMoonCode(Data.Code.ByName("gml_Object_oSunMoon_Step_0"));
PatchSunMoonCode(Data.Code.ByName("gml_Object_oOfficeSunPoster_Step_0"));
PatchSunMoonCode(Data.Code.ByName("gml_Object_oSleepyMoon_Create_0"));

// ==============================================================
// 8. HOOK NATIVO EN REVIEW ANIMATRONICS (END STEP Y DRAW GUI EN oTitleMenu V27)
// ==============================================================
// oTitleMenu no tiene EndStep ni DrawGUI nativamente, por lo que crearlos
// compila de forma limpia y directa al 100% sin depender de descompilacion.

string reviewEndStepCode = @"// Murray Moose en Review Animatronics (EndStep V27)
if (variable_instance_exists(id, ""Options"") && is_array(Options))
{
    var _has_back = false;
    var _has_resume = false;
    var _found_murray = false;
    var _back_idx = -1;
    
    for (var _i = 0; _i < array_length(Options); _i++)
    {
        var _opt = Options[_i];
        if (_opt == ""Back"") { _has_back = true; _back_idx = _i; }
        if (_opt == ""Resume"" || _opt == ""Restart Night"" || _opt == ""Quit to Menu"" || _opt == ""Quit Game"") _has_resume = true;
        if (_opt == ""Remap Controls"" || _opt == ""Graphics"" || _opt == ""Audio"" || _opt == ""Gameplay"" || _opt == ""Delete Save Data"") _has_resume = true;
        if (_opt == ""Review Upgrades"" || _opt == ""Restore Defaults"") _has_resume = true;
        if (_opt == ""Murray Moose"") _found_murray = true;
    }
    
    // Si estamos en la pantalla de Review Animatronics
    if (_has_back && !_has_resume && !_found_murray)
    {
        var _murray_active = false;
        if (variable_global_exists(""Murray_AI"") && global.Murray_AI > 0) _murray_active = true;
        if (variable_global_exists(""EndlessMode"") && global.EndlessMode) _murray_active = true;
        if (variable_global_exists(""FourLayersDown"") && global.FourLayersDown) _murray_active = true;
        var _m_obj = asset_get_index(""oMurray_BackWindow"");
        if (_m_obj != -1 && instance_number(_m_obj) > 0) _murray_active = true;
        
        if (_murray_active)
        {
            var _pos = (_back_idx != -1) ? _back_idx : array_length(Options);
            array_insert(Options, _pos, ""Murray Moose"");
            
            if (variable_instance_exists(id, ""DescriptionList"") && is_array(DescriptionList))
            {
                while (array_length(DescriptionList) < _pos)
                {
                    array_push(DescriptionList, "");
                }
                array_insert(DescriptionList, _pos, ""He knocks on the right office window. Raise the window blind to spot him, click his sunglasses to knock them off, and shine your flashlight to scare him away."");
            }
            
            if (variable_instance_exists(id, ""AmountOfOptions""))
            {
                AmountOfOptions = array_length(Options);
            }
        }
    }
}";

string reviewDrawGuiCode = @"// Dibujar descripcion de Murray en Review Animatronics (DrawGUI V27)
if (variable_instance_exists(id, ""Options"") && is_array(Options) && variable_instance_exists(id, ""Select""))
{
    var _has_back = false;
    var _has_resume = false;
    for (var _i = 0; _i < array_length(Options); _i++)
    {
        var _opt = Options[_i];
        if (_opt == ""Back"") _has_back = true;
        if (_opt == ""Resume"" || _opt == ""Restart Night"" || _opt == ""Quit to Menu"" || _opt == ""Quit Game"") _has_resume = true;
    }
    
    if (_has_back && !_has_resume && Select >= 0 && Select < array_length(Options))
    {
        if (Options[Select] == ""Murray Moose"")
        {
            var fn = asset_get_index(""fTiny"");
            if (fn == -1) fn = asset_get_index(""fDefault"");
            if (fn != -1) draw_set_font(fn);
            draw_set_color(c_white);
            draw_set_halign(fa_left);
            draw_set_valign(fa_top);
            draw_text_ext(76, 305, ""He knocks on the right office window. Raise the window blind to spot him, click his sunglasses to knock them off, and shine your flashlight to scare him away."", 14, 490);
        }
    }
}";

UndertaleGameObject objTitleMenu = Data.GameObjects.ByName("oTitleMenu");
if (objTitleMenu != null)
{
    importGroup.QueueReplace(objTitleMenu.EventHandlerFor(EventType.Step, EventSubtypeStep.EndStep, Data), reviewEndStepCode);
    importGroup.QueueReplace(objTitleMenu.EventHandlerFor(EventType.Draw, EventSubtypeDraw.DrawGUI, Data), reviewDrawGuiCode);
}

importGroup.Import();

// 9. DEPLOY EN MOD GALLERY Y GESTIÓN ESTRICTA DE AUDIO (OGG V23)
// ==============================================================
try
{
    string gameDir = @"C:\Juegos\frickbears3-4ld-v2.0.2";
    try
    {
        if (!string.IsNullOrEmpty(FilePath))
        {
            string dir = Path.GetDirectoryName(FilePath);
            if (!string.IsNullOrEmpty(dir) && Directory.Exists(dir))
            {
                gameDir = dir;
            }
        }
    }
    catch { }

    string dstAddons = Path.Combine(gameDir, "addons", "Murray");
    Directory.CreateDirectory(dstAddons);

    string srcAddons = @"D:\Frickbears-mod\addons\Murray";
    if (Directory.Exists(srcAddons))
    {
        if (File.Exists(Path.Combine(srcAddons, "extras_info.txt")))
            File.Copy(Path.Combine(srcAddons, "extras_info.txt"), Path.Combine(dstAddons, "extras_info.txt"), true);
        if (File.Exists(Path.Combine(srcAddons, "portrait.png")))
            File.Copy(Path.Combine(srcAddons, "portrait.png"), Path.Combine(dstAddons, "portrait.png"), true);
    }

    // Copiar sTile_Murray_0.png (48x48) como icon.png para Mod Gallery
    string iconSrc = Path.Combine(assetsFolder, "sTile_Murray_0.png");
    if (File.Exists(iconSrc))
    {
        File.Copy(iconSrc, Path.Combine(dstAddons, "icon.png"), true);
    }

    // AUDIO ESTRICTO: ÚNICAMENTE snd_murray_knock.ogg permitido
    string knockSrc = @"D:\Frickbears-mod\Murray_Sounds_To_Import\snd_murray_knock.ogg";
    if (File.Exists(knockSrc))
    {
        File.Copy(knockSrc, Path.Combine(gameDir, "snd_murray_knock.ogg"), true);
    // Copiar snd_murray_jumpscare.ogg (tonada alegre extraída del video oficial)
    string scareSrc = @"D:\Frickbears-mod\Murray_Sounds_To_Import\snd_murray_jumpscare.ogg";
    if (File.Exists(scareSrc))
    {
        File.Copy(scareSrc, Path.Combine(gameDir, "snd_murray_jumpscare.ogg"), true);
    }

    string localAppDataDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Frickbears3");
    if (Directory.Exists(localAppDataDir))
    {
        if (File.Exists(knockSrc)) File.Copy(knockSrc, Path.Combine(localAppDataDir, "snd_murray_knock.ogg"), true);
        if (File.Exists(scareSrc)) File.Copy(scareSrc, Path.Combine(localAppDataDir, "snd_murray_jumpscare.ogg"), true);
    }
    }

    // PURGAR CUALQUIER LÍNEA DE VOZ O AUDIO EXTRA OBSOLETO
    string[] obsoleteAudios = new string[] {
        "murray_knock.ogg", "murray_amigable.ogg", "murray_knock2.ogg",
        "murray_voice.ogg", "snd_murray_voice.ogg"
    };
    foreach (string badAudio in obsoleteAudios)
    {
        string p = Path.Combine(gameDir, badAudio);
        if (File.Exists(p)) { try { File.Delete(p); } catch { } }
    }

    // Purgar skins/ por Invariante #6
    string badRender = Path.Combine(dstAddons, "FB_Murray_Moose.png");
    if (File.Exists(badRender)) { try { File.Delete(badRender); } catch { } }

    string dstSkins = Path.Combine(gameDir, "skins", "Murray");
    if (Directory.Exists(dstSkins)) { try { Directory.Delete(dstSkins, true); } catch { } }
}
catch (Exception)
{
}

ScriptMessage("¡MURRAY MOOSE V27 INSTALADO CON ÉXITO!\n\n" +
              "CORRECCIONES APLICADAS EN V27:\n" +
              "1. Menú de Pausa (ESC) 100% Protegido: Audio de jumpscare en LocalAppData/working_dir, sin voz aleatoria en GameOver y Murray en Review Animatronics.\n" +
              "2. Jumpscare Centrado Verticalmente: Murray bajado a la altura de los ojos del jugador (+85px) para centrado cinematográfico.\n" +
              "3. Mecánica Elegante de Persiana: Al levantarla el jugador, Murray la sostiene arriba cómodamente; al ahuyentarlo con la linterna cae sola.\n" +
              "4. Audio y Animación: Música alegre del jumpscare (snd_murray_jumpscare.ogg) y animación de cuerpo (frames 0..5).\n\n" +
              "¡Presiona Ctrl+S en UndertaleModTool para guardar data.win!");

// ==============================================================
// 10. CLASES AUXILIARES DEL EMPAQUETADOR DE TEXTURAS (Packer V23)
// ==============================================================
public class TextureInfo
{
    public string Source;
    public int Width;
    public int Height;
    public int TargetX;
    public int TargetY;
    public int BoundingWidth;
    public int BoundingHeight;
    public MagickImage Image;
}

public enum SpriteType { Sprite, Background, Font, Unknown }
public enum SplitType { Horizontal, Vertical }
public enum BestFitHeuristic { Area, MaxOneAxis }

public struct Rect
{
    public int X { get; set; }
    public int Y { get; set; }
    public int Width { get; set; }
    public int Height { get; set; }
}

public class Node
{
    public Rect Bounds;
    public TextureInfo Texture;
    public SplitType SplitType;
}

public class Atlas
{
    public int Width;
    public int Height;
    public List<Node> Nodes;
}

public class Packer
{
    public List<TextureInfo> SourceTextures = new List<TextureInfo>();
    public StringWriter Log = new StringWriter();
    public StringWriter Error = new StringWriter();
    public int Padding;
    public int AtlasSize;
    public bool DebugMode;
    public BestFitHeuristic FitHeuristic;
    public List<Atlas> Atlasses;

    public void Process(string _SourceDir, string _Pattern, int _AtlasSize, int _Padding, bool _DebugMode)
    {
        Padding = _Padding;
        AtlasSize = _AtlasSize;
        DebugMode = _DebugMode;
        ScanForTextures(_SourceDir, _Pattern);
        List<TextureInfo> textures = SourceTextures.ToList();
        Atlasses = new List<Atlas>();
        while (textures.Count > 0)
        {
            Atlas atlas = new Atlas() { Width = _AtlasSize, Height = _AtlasSize };
            List<TextureInfo> leftovers = LayoutAtlas(textures, atlas);
            if (leftovers.Count == 0)
            {
                while (leftovers.Count == 0)
                {
                    atlas.Width /= 2;
                    atlas.Height /= 2;
                    leftovers = LayoutAtlas(textures, atlas);
                }
                atlas.Width = (atlas.Width == 0) ? 1 : atlas.Width * 2;
                atlas.Height = (atlas.Height == 0) ? 1 : atlas.Height * 2;
                leftovers = LayoutAtlas(textures, atlas);
            }
            Atlasses.Add(atlas);
            textures = leftovers;
        }
    }

    public void SaveAtlasses(string _Destination)
    {
        int atlasCount = 0;
        string prefix = Path.Join(Path.GetDirectoryName(_Destination), Path.GetFileNameWithoutExtension(_Destination));
        using StreamWriter tw = new StreamWriter(_Destination);
        tw.WriteLine("source_tex, atlas_tex, x, y, width, height");
        foreach (Atlas atlas in Atlasses)
        {
            string atlasName = $"{prefix}{atlasCount:000}.png";
            using (MagickImage img = CreateAtlasImage(atlas))
            {
                TextureWorker.SaveImageToFile(img, atlasName);
            }
            foreach (Node n in atlas.Nodes)
            {
                if (n.Texture != null)
                {
                    tw.WriteLine($"{n.Texture.Source}, {atlasName}, {n.Bounds.X}, {n.Bounds.Y}, {n.Bounds.Width}, {n.Bounds.Height}");
                }
            }
            atlasCount++;
        }
    }

    private void ScanForTextures(string _Path, string _Wildcard)
    {
        DirectoryInfo di = new(_Path);
        FileInfo[] files = di.GetFiles(_Wildcard, SearchOption.AllDirectories);
        foreach (FileInfo fi in files)
        {
            (int width, int height) = TextureWorker.GetImageSizeFromFile(fi.FullName);
            if (width == -1 || height == -1) continue;

            if (width <= AtlasSize && height <= AtlasSize)
            {
                TextureInfo ti = new();
                MagickImage img = new(fi.FullName);
                ti.Source = fi.FullName;
                ti.BoundingWidth = (int)img.Width;
                ti.BoundingHeight = (int)img.Height;
                ti.TargetX = 0;
                ti.TargetY = 0;
                ti.Width = (int)img.Width;
                ti.Height = (int)img.Height;
                ti.Image = img;
                SourceTextures.Add(ti);
            }
        }
    }

    private void HorizontalSplit(Node _ToSplit, int _Width, int _Height, List<Node> _List)
    {
        Node n1 = new Node();
        n1.Bounds.X = _ToSplit.Bounds.X + _Width + Padding;
        n1.Bounds.Y = _ToSplit.Bounds.Y;
        n1.Bounds.Width = _ToSplit.Bounds.Width - _Width - Padding;
        n1.Bounds.Height = _Height;
        n1.SplitType = SplitType.Vertical;

        Node n2 = new Node();
        n2.Bounds.X = _ToSplit.Bounds.X;
        n2.Bounds.Y = _ToSplit.Bounds.Y + _Height + Padding;
        n2.Bounds.Width = _ToSplit.Bounds.Width;
        n2.Bounds.Height = _ToSplit.Bounds.Height - _Height - Padding;
        n2.SplitType = SplitType.Horizontal;

        if (n1.Bounds.Width > 0 && n1.Bounds.Height > 0) _List.Add(n1);
        if (n2.Bounds.Width > 0 && n2.Bounds.Height > 0) _List.Add(n2);
    }

    private void VerticalSplit(Node _ToSplit, int _Width, int _Height, List<Node> _List)
    {
        Node n1 = new Node();
        n1.Bounds.X = _ToSplit.Bounds.X + _Width + Padding;
        n1.Bounds.Y = _ToSplit.Bounds.Y;
        n1.Bounds.Width = _ToSplit.Bounds.Width - _Width - Padding;
        n1.Bounds.Height = _ToSplit.Bounds.Height;
        n1.SplitType = SplitType.Vertical;

        Node n2 = new Node();
        n2.Bounds.X = _ToSplit.Bounds.X;
        n2.Bounds.Y = _ToSplit.Bounds.Y + _Height + Padding;
        n2.Bounds.Width = _Width;
        n2.Bounds.Height = _ToSplit.Bounds.Height - _Height - Padding;
        n2.SplitType = SplitType.Horizontal;

        if (n1.Bounds.Width > 0 && n1.Bounds.Height > 0) _List.Add(n1);
        if (n2.Bounds.Width > 0 && n2.Bounds.Height > 0) _List.Add(n2);
    }

    private TextureInfo FindBestFitForNode(Node _Node, List<TextureInfo> _Textures)
    {
        TextureInfo bestFit = null;
        float nodeArea = _Node.Bounds.Width * _Node.Bounds.Height;
        float maxCriteria = 0.0f;
        foreach (TextureInfo ti in _Textures)
        {
            if (ti.Width <= _Node.Bounds.Width && ti.Height <= _Node.Bounds.Height)
            {
                float textureArea = ti.Width * ti.Height;
                float coverage = textureArea / nodeArea;
                if (coverage > maxCriteria)
                {
                    maxCriteria = coverage;
                    bestFit = ti;
                }
            }
        }
        return bestFit;
    }

    private List<TextureInfo> LayoutAtlas(List<TextureInfo> _Textures, Atlas _Atlas)
    {
        List<Node> freeList = new List<Node>();
        List<TextureInfo> textures = _Textures.ToList();
        _Atlas.Nodes = new List<Node>();
        Node root = new Node();
        root.Bounds.Width = _Atlas.Width;
        root.Bounds.Height = _Atlas.Height;
        root.SplitType = SplitType.Horizontal;
        freeList.Add(root);

        while (freeList.Count > 0 && textures.Count > 0)
        {
            Node node = freeList[0];
            freeList.RemoveAt(0);
            TextureInfo bestFit = FindBestFitForNode(node, textures);
            if (bestFit != null)
            {
                if (node.SplitType == SplitType.Horizontal)
                    HorizontalSplit(node, bestFit.Width, bestFit.Height, freeList);
                else
                    VerticalSplit(node, bestFit.Width, bestFit.Height, freeList);

                node.Texture = bestFit;
                node.Bounds.Width = bestFit.Width;
                node.Bounds.Height = bestFit.Height;
                textures.Remove(bestFit);
            }
            _Atlas.Nodes.Add(node);
        }
        return textures;
    }

    private MagickImage CreateAtlasImage(Atlas _Atlas)
    {
        MagickImage img = new(MagickColors.Transparent, (uint)_Atlas.Width, (uint)_Atlas.Height);
        foreach (Node n in _Atlas.Nodes)
        {
            if (n.Texture is not null)
            {
                MagickImage sourceImg = n.Texture.Image;
                using IMagickImage<byte> resizedSourceImg = TextureWorker.ResizeImage(sourceImg, n.Bounds.Width, n.Bounds.Height);
                img.Composite(resizedSourceImg, n.Bounds.X, n.Bounds.Y, CompositeOperator.Copy);
            }
        }
        return img;
    }
}
