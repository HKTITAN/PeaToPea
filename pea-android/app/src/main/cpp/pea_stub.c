/* Stub when libpea_core.a is not built; replaced by real pea_core when linked. */
#include <stddef.h>
#include <stdint.h>

uint8_t pea_core_version(void) { return 1; }
void* pea_core_create(void) { return NULL; }
void pea_core_destroy(void* h) { (void)h; }
int pea_core_device_id(void* h, void* out_buf, size_t out_len) { (void)h; (void)out_buf; (void)out_len; return -1; }
int pea_core_on_request(void* h, const void* url, size_t url_len, uint64_t range_start, uint64_t range_end, void* out_buf, size_t out_buf_len) { (void)h; (void)url; (void)url_len; (void)range_start; (void)range_end; (void)out_buf; (void)out_buf_len; return -1; }
int pea_core_peer_joined(void* h, const void* device_id_16, const void* public_key_32) { (void)h; (void)device_id_16; (void)public_key_32; return -1; }
int pea_core_peer_left(void* h, const void* device_id_16, void* out_buf, size_t out_buf_len) { (void)h; (void)device_id_16; (void)out_buf; (void)out_buf_len; return 0; }
int pea_core_on_message_received(void* h, const void* peer_id_16, const void* msg, size_t msg_len, void* out_buf, size_t out_buf_len) { (void)h; (void)peer_id_16; (void)msg; (void)msg_len; (void)out_buf; (void)out_buf_len; return -1; }
int pea_core_on_chunk_received(void* h, const void* transfer_id_16, uint64_t start, uint64_t end, const void* hash_32, const void* payload, size_t payload_len, void* out_buf, size_t out_buf_len) { (void)h; (void)transfer_id_16; (void)start; (void)end; (void)hash_32; (void)payload; (void)payload_len; (void)out_buf; (void)out_buf_len; return -1; }
int pea_core_tick(void* h, void* out_buf, size_t out_buf_len) { (void)h; (void)out_buf; (void)out_buf_len; return 0; }
