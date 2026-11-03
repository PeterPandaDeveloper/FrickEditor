if (variable_instance_exists(id, "MurrayBody") && MurrayBody != noone && instance_exists(MurrayBody)) {
    instance_destroy(MurrayBody);
    MurrayBody = noone;
}
if (variable_instance_exists(id, "MurrayHead") && MurrayHead != noone && instance_exists(MurrayHead)) {
    instance_destroy(MurrayHead);
    MurrayHead = noone;
}
if (variable_instance_exists(id, "MurrayGlasses") && MurrayGlasses != noone && instance_exists(MurrayGlasses)) {
    instance_destroy(MurrayGlasses);
    MurrayGlasses = noone;
}
