// ==============================================================
// Murray Moose - Custom Night Tile & Page Switcher (Create V21)
// Configuración exacta para Página 2 nativa de Frickbear's 3
// ==============================================================
depth = -1000;
if (!variable_global_exists("Murray_AI")) {
    global.Murray_AI = 0;
}
if (!variable_global_exists("CustomNight_Page")) {
    global.CustomNight_Page = 0; // 0 = Página 1 (Vanilla), 1 = Página 2 (Murray)
}

// Botón compacto perfectamente cuadrado debajo de Columnas 6 y 7 (344, 326, 110, 26)
// Margen limpio de 11px respecto a la barra de scroll (X=465) y 8px respecto al borde inferior (Y=360)
btn_page_x = 344;
btn_page_y = 326;
btn_page_w = 110;
btn_page_h = 26;

// Coordenadas exactas de la Primera Casilla (Fila 0, Columna 0 de la grilla)
tile_x = 32;
tile_y = 12;
