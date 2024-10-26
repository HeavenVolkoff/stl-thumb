#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <openssl/md5.h>

#include "stl_thumb.h"

int main(int argc, char *argv[]) {
  if (argc < 2) {
    fprintf(stderr, "Usage: %s <filename>\n", argv[0]);
    return 1;
  }

  const char *filename = argv[1];
  int width = 1024;
  int height = 1024;
  float cam_fov_deg = 45.0F;
  float cam_position[3] = {2.0F, -4.0F, 2.0F};
  int img_size = width * height * 4;

  uint8_t *output_buf = (uint8_t *)malloc(img_size);
  if (output_buf == NULL) {
    fprintf(stderr, "Failed to allocate memory\n");
    return 1;
  }

  render_to_buffer(filename, width, height, cam_fov_deg, cam_position, 4, false, output_buf);

  // Calculate MD5 hash
  unsigned char result[MD5_DIGEST_LENGTH];
  MD5(output_buf, img_size, result);

  // Print MD5 hash
  printf("MD5: ");
  for (int i = 0; i < MD5_DIGEST_LENGTH; i++) {
    printf("%02x", result[i]);
  }
  printf("\n");

  free(output_buf);

  return EXIT_SUCCESS;
}
