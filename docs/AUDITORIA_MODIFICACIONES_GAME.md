# 📋 NOTA TÉCNICA DE MODIFICACIONES Y AUDITORÍA DE DATA.WIN

**Juego Destino:** *Five Nights at Frickbear's 3 + Four Layers Down (v2.0.2)*  
**Herramienta de Inyección:** UndertaleModTool (UTMT) & FrickEditor Studio  
**Fecha de Auditoría:** 13 de Noviembre de 2026  
**Estado de Integridad:** ✅ Seguro, 100% Desacoplado y Libre de Corrupción  

---

## 1. Resumen Ejecutivo
Esta auditoría detalla con precisión quirúrgica qué elementos del juego han sido modificados, qué objetos nuevos fueron creados, qué código vanilla fue hookeado y cómo opera la arquitectura en tiempo de ejecución.

Asimismo, clarifica por qué los objetos nativos como `oMarionette` (Puppet) y `oPhantoms` (The Phantoms) no han sufrido ninguna alteración por parte de los mods y cómo funciona su comportamiento nativo en *Four Layers Down*.

---

## 2. Objetos y Elementos Nuevos Creados (Murray Moose)

| Objeto / Asset | Tipo | Ubicación / Coordenadas | Función y Comportamiento |
| :--- | :--- | :--- | :--- |
| **`oMurray_BackWindow`** | Objeto GML | Room `Office` (1226, 1085) | Animatrónico en la ventana derecha. Toca el cristal a intervalos aleatorios (`knock_timer`). Si el jugador sube la persiana derecha, Murray aparece acechando; al hacer clic en sus gafas oscuras, estas caen (`knocked_glasses = true`), momento en el que el jugador debe encender la linterna para ahuyentarlo. |
| **`oMurray_CustomNightTile`** | Objeto GML | Room `CustomNightMenu` | Tile interactivo de 48x48 píxeles. Dibuja el retrato a color (`AI > 0`) o gris (`AI == 0`). Se posiciona dinámicamente sumando `TreeY` de Rodney (`Index 46`) para respetar el scroll del menú sin desalinearse. |
| **`oMurray_DeathScreen`** | Objeto GML | Capa de GameOver | Pantalla de muerte personalizada. Renderiza el sprite de Murray en silueta roja carmesí escalado a 3.2x y reproduce su jumpscare de derrota cuando `global.MurrayMoose_Kill == true`. |
| **Sprites de Animación** | Texturas | Texture Pages | `sMurray_body` (torso), `sMurray_head` (cabeza y expresiones), `sMurray_sunglasses` (gafas oscuras), `sMurray_Jumpscare` (atlas 640x640), `sTile_Murray_0` y `sTile_Murray_1` (48x48). |
| **Sonidos OGG** | Audio OGG | `%LOCALAPPDATA%\Frickbears3\` y carpeta del juego | `snd_murray_knock.ogg` (golpe en ventana), `snd_murray_jumpscare.ogg` (chillido de susto), `murray_amigable.ogg` (audio de prueba/salvamento). |

---

## 3. Código Vanilla Modificado e Inyectado (Hooks)

Para que un animatrónico custom funcione en Frickbear's 3 sin romper el juego original, se aplicaron 3 hooks quirúrgicos:

### A. Hook en `oGameOver` (`Create_0` y `Draw_0`)
* **Qué hace:** Al morir el jugador, `oGameOver` comprueba si `global.MurrayMoose_Kill == true`.
* **Cómo lo hace:** Si es Murray quien mató al guardia, delega el renderizado visual a `oMurray_DeathScreen` para dibujar la silueta roja a 3.2x con la frase de muerte. Si es cualquier otro animatrónico vanilla, el juego ejecuta su flujo habitual sin alteración.

### B. Desacoplamiento de Persianas en `oSunMoon`, `oOfficeSunPoster` y `oSleepyMoon`
* **Invariante #3:** En el código vanilla, Sun y Moon forzaban la apertura de `oWindowBlinds.Open = true`.
* **Qué se modificó:** Se reemplazó la búsqueda global indiscriminada por `instance_nearest(1226, 1085, oWindowBlinds)` para desacoplar la persiana derecha de la izquierda.
* **Beneficio:** Permite cerrar la persiana derecha para defenderse de Murray Moose sin provocar la muerte instantánea del jugador a manos de Sun.

### C. Inserción Dinámica en `oTitleMenu` (`EndStep` y `DrawGUI`)
* **Qué hace:** Agrega a Murray Moose a la pantalla oficial de **"Review Animatronics"**.
* **Cómo lo hace de forma segura:** En lugar de modificar los arrays estáticos del binario (lo que corrompería la memoria del menú), detecta dinámicamente si el jugador está en la pantalla de opciones de Review y realiza un `array_insert(Options, pos, "Murray Moose")` y su descripción correspondiente en tiempo de ejecución. Al salir, la memoria vuelve a su estado nativo.

---

## 4. Auditoría de Objetos Vanilla NO Modificados (100% Intactos)

### A. Marionette / Puppet (`oMarionette` y `oMarionette_WarningDrawer`)
* **Estado:** **100% Intacto / Sin alteraciones.**
* **Comportamiento Nativo:** En la sala `Office`, la instancia `100038` de `oMarionette` viene pre-colocada por los desarrolladores de Frickbear's. No contiene `instance_destroy()` en `Create_0` porque la caja de música es un **peligro pasivo permanente del entorno**.

### B. The Phantoms (`oPhantoms`) en Four Layers Down (4LD)
* **Estado:** **100% Intacto / Sin alteraciones.**
* **Comportamiento Nativo:** En `Office`, la instancia `100060` de `oPhantoms` está pre-colocada. Los fantasmas son controlados mediante las variables de agresión `AmountOfPhantoms` y `Aggro`.
* **Por qué aparecen en el selector de 4LD:** En Four Layers Down / Endless Mode, la tienda te permite comprar/añadir "The Phantoms" no para duplicarlos, sino para **activar más miembros del grupo de fantasmas** (Phantom BB, Phantom Foxy, Phantom Chica, Phantom Freddy, Phantom Mangle) y acelerar sus apariciones en las cámaras.

---

## 5. Reglas Invariantes de Seguridad Cumplidas
1. **Versionado Estricto:** Cada iteración se compila en un script independiente (`Instalar_Murray_V1.csx` ... `V27.csx`).
2. **Seguridad en CleanUp:** Todas las variables de instancia se declaran antes de cualquier condición en `Create_0` y se validan con `variable_instance_exists()`.
3. **Persianas y Sun/Moon:** Nunca se fuerza `ForcedOpen` en la persiana derecha.
4. **Scroll en Custom Night:** Se respeta la fórmula `draw_y = base_y + rodney.TreeY`.
5. **Separación de Addons:** Metadatos exclusivos en `addons/Murray/extras_info.txt`, evitando la carpeta `skins/` para prevenir fallos con `string_to_int_before_underscore`.
